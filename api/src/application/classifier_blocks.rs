use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use apalis::prelude::*;
use bb8::PooledConnection;
use chrono::{Datelike, Duration, NaiveDate, Utc};
use diesel::prelude::*;
use diesel::upsert::excluded;
use diesel_async::AsyncPgConnection;
use diesel_async::RunQueryDsl;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use regex::{Captures, Regex};
use sprintf::sprintf;

use crate::application::document_index_documents::enqueue_document_index_document_updates;
use crate::application::document_metadatas::{NewDocumentMetadata, document_metadatas_upsert};
use crate::application::documents::get_document_view;
use crate::application::documents::update_document;
use crate::domain::classifier_blocks::ClassifierModifier;
use crate::domain::classifier_blocks::{ClassifierBlock, ClassifierChildRule, ClassifierPattern};
use crate::domain::documents::DocumentChangeset;
use crate::domain::documents::DocumentView;
use crate::schema::document_types;
use crate::schema::{
    cabinet_documents, cabinets, classifier_blocks, document_file_ocr_pages, document_file_pages,
    document_files, metadata_types, tag_documents, tags,
};
use crate::shared::app_state::AppState;
use crate::shared::util::JobResult;

#[derive(Insertable)]
#[diesel(table_name = tag_documents)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct InsertableSuggestedTagDocument {
    tag_id: i64,
    document_id: i64,
    updated_by: i64,
}

#[derive(Insertable)]
#[diesel(table_name = cabinet_documents)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct InsertableSuggestedCabinetDocument {
    cabinet_id: i64,
    document_id: i64,
    updated_by: i64,
}

#[derive(Debug)]
pub enum PatternMatch<'a> {
    None,
    Text(Captures<'a>),
    Metadata,
}

pub async fn classify_document(
    document_id: i64,
    user_id: i64,
    state: Data<Arc<AppState>>,
) -> Result<(), Error> {
    classify_document_inner(document_id, user_id, state)
        .await
        .map_err(Into::into)
}

async fn classify_document_inner(
    document_id: i64,
    user_id: i64,
    state: Data<Arc<AppState>>,
) -> JobResult<()> {
    tracing::info!(document_id, "classification: classifying document");

    let mut db = state.db_pool.get().await?;

    // Load the document view from the database
    let document_view = get_document_view(&mut db, document_id).await?;

    // Load the document text, trying content text first, then falling back to OCR.
    // Join all of the pages together into a single string.
    let document_text = load_document_text(&mut db, document_id).await?;

    // Load all of the classifier blocks from the database, ordered by their "order" field.
    let classifier_blocks = load_classifier_blocks(&mut db).await?;

    let mut computed_actions = HashMap::new();

    // Iterate over each classifier block and pull out it's ClassifierRules.
    // Try to match the document against the ClassifierRules, starting with the match_patterns.
    // call does_document_match_pattern for each match_pattern
    for classifier_block in classifier_blocks {
        let rules = &classifier_block.rules.0;

        let pattern_match =
            find_first_match(&document_view, &document_text, &rules.match_patterns)?;

        // If any of the match_patterns match, then we can apply the match_actions to the document,
        // and then move on to the child_rules application.
        if pattern_match.is_some() {
            tracing::debug!(document_id, classifier_block_id = classifier_block.id, "classification: block matched");
            apply_match_actions(&mut computed_actions, &rules.match_actions);
            apply_child_rules(
                &document_view,
                &document_text,
                &mut computed_actions,
                &rules.child_rules,
            )?;

            if !rules.continue_after_match {
                break;
            }
        }
    }

    // Finally, we will have a set of computed actions that we want to apply to the document.
    // Iterate over all of the computed actions.
    tracing::debug!(document_id, ?computed_actions, "classification: computed actions");
    let mut document_type_id: Option<i64> = None;
    let mut title: Option<String> = None;
    let mut computed_metadata: HashMap<String, String> = HashMap::new();
    for (key, value) in computed_actions {
        match key.as_str() {
            "_suggested_doctype" => {
                tracing::info!(
                    document_id,
                    document_type = value,
                    "classification: suggested document_type"
                );
                let doctype = document_types::table
                    .filter(document_types::slug.eq(value))
                    .select(document_types::id)
                    .first::<i64>(&mut db)
                    .await?;
                document_type_id = Some(doctype);
            }
            "_suggested_filename" => {
                tracing::info!(document_id, title = value, "classification: suggested title");
                title = Some(value);
            }
            "_suggested_tags" => {
                tracing::info!(document_id, tags = value, "classification: suggested tags");
                let slugs = parse_slug_list(&value);
                apply_suggested_tags(&mut db, document_id, user_id, &slugs).await?;
            }
            "_suggested_cabinets" => {
                tracing::info!(document_id, cabinets = value, "classification: suggested cabinet");
                let slugs = parse_slug_list(&value);
                apply_suggested_cabinets(&mut db, document_id, user_id, &slugs).await?;
            }
            _ if key.starts_with('_') => continue,
            _ => {
                // Otherwise, this is metadata that we want to apply to the document.
                computed_metadata.insert(key, value);
                ()
            }
        }
    }

    // Now, we need to update the document title and type.
    if document_type_id.is_some() || title.is_some() {
        update_document(
            user_id,
            &mut db,
            document_id,
            DocumentChangeset {
                title,
                document_type_id,
            },
        )
        .await?;
    }

    // Finally, we have the metadata that we want to apply to the document.
    // It is time to upsert it.
    tracing::info!(
        document_id,
        ?computed_metadata,
        "classification: computed metadata to apply to document"
    );

    if !computed_metadata.is_empty() {
        let metadata_slugs: Vec<String> = computed_metadata.keys().cloned().collect();
        let metadata_rows: Vec<(String, i64)> = metadata_types::table
            .filter(metadata_types::slug.eq_any(&metadata_slugs))
            .select((metadata_types::slug, metadata_types::id))
            .load::<(String, i64)>(&mut db)
            .await?;

        let metadata_type_ids: HashMap<String, i64> = metadata_rows.into_iter().collect();
        let mut metadata_input = Vec::new();

        for (slug, value) in computed_metadata {
            if let Some(metadata_type_id) = metadata_type_ids.get(&slug) {
                metadata_input.push(NewDocumentMetadata {
                    metadata_type_id: *metadata_type_id,
                    value,
                });
            } else {
                tracing::error!(
                    document_id,
                    slug,
                    "classification: classifier produced unknown metadata slug"
                );
            }
        }

        if !metadata_input.is_empty() {
            document_metadatas_upsert(user_id, &mut db, document_id, metadata_input).await?;
        }
    }

    enqueue_document_index_document_updates(document_id, (*state).clone()).await?;

    Ok(())
}

fn parse_slug_list(value: &str) -> Vec<String> {
    let mut seen = HashSet::new();

    value
        .split(',')
        .map(str::trim)
        .filter(|slug| !slug.is_empty())
        .filter(|slug| seen.insert((*slug).to_string()))
        .map(str::to_string)
        .collect()
}

async fn apply_suggested_tags(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    document_id: i64,
    user_id: i64,
    slugs: &[String],
) -> JobResult<()> {
    if slugs.is_empty() {
        return Ok(());
    }

    let rows: Vec<(i64, String)> = tags::table
        .filter(tags::slug.eq_any(slugs))
        .select((tags::id, tags::slug))
        .load::<(i64, String)>(db)
        .await?;

    let found_slugs: HashSet<String> = rows.iter().map(|(_, slug)| slug.clone()).collect();
    for slug in slugs {
        if !found_slugs.contains(slug) {
            tracing::warn!(document_id, slug, "classification: suggested tag not found");
        }
    }

    let values: Vec<InsertableSuggestedTagDocument> = rows
        .into_iter()
        .map(|(tag_id, _)| InsertableSuggestedTagDocument {
            tag_id,
            document_id,
            updated_by: user_id,
        })
        .collect();

    if values.is_empty() {
        return Ok(());
    }

    diesel::insert_into(tag_documents::table)
        .values(&values)
        .on_conflict((tag_documents::tag_id, tag_documents::document_id))
        .do_update()
        .set((
            tag_documents::updated_by.eq(excluded(tag_documents::updated_by)),
            tag_documents::updated_at.eq(Utc::now()),
        ))
        .execute(db)
        .await?;

    Ok(())
}

async fn apply_suggested_cabinets(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    document_id: i64,
    user_id: i64,
    slugs: &[String],
) -> JobResult<()> {
    if slugs.is_empty() {
        return Ok(());
    }

    let rows: Vec<(i64, String)> = cabinets::table
        .filter(cabinets::slug.eq_any(slugs))
        .select((cabinets::id, cabinets::slug))
        .load::<(i64, String)>(db)
        .await?;

    let found_slugs: HashSet<String> = rows.iter().map(|(_, slug)| slug.clone()).collect();
    for slug in slugs {
        if !found_slugs.contains(slug) {
            tracing::warn!(document_id, slug, "classification: suggested cabinet not found");
        }
    }

    let values: Vec<InsertableSuggestedCabinetDocument> = rows
        .into_iter()
        .map(|(cabinet_id, _)| InsertableSuggestedCabinetDocument {
            cabinet_id,
            document_id,
            updated_by: user_id,
        })
        .collect();

    if values.is_empty() {
        return Ok(());
    }

    diesel::insert_into(cabinet_documents::table)
        .values(&values)
        .on_conflict((
            cabinet_documents::cabinet_id,
            cabinet_documents::document_id,
        ))
        .do_update()
        .set((
            cabinet_documents::updated_by.eq(excluded(cabinet_documents::updated_by)),
            cabinet_documents::updated_at.eq(Utc::now()),
        ))
        .execute(db)
        .await?;

    Ok(())
}

async fn load_document_text(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    document_id: i64,
) -> JobResult<String> {
    let text_pages: Vec<Option<String>> = document_file_pages::table
        .inner_join(
            document_files::table.on(document_files::id.eq(document_file_pages::document_file_id)),
        )
        .filter(document_files::document_id.eq(document_id))
        .order((
            document_files::id.asc(),
            document_file_pages::page_number.asc(),
        ))
        .select(document_file_pages::text_content)
        .load::<Option<String>>(db)
        .await?;

    let text_content = join_non_empty_pages(text_pages);
    if !text_content.is_empty() {
        return Ok(text_content);
    }

    let ocr_pages: Vec<Option<String>> = document_file_ocr_pages::table
        .inner_join(
            document_files::table
                .on(document_files::id.eq(document_file_ocr_pages::document_file_id)),
        )
        .filter(document_files::document_id.eq(document_id))
        .order((
            document_files::id.asc(),
            document_file_ocr_pages::page_number.asc(),
        ))
        .select(document_file_ocr_pages::ocr_content)
        .load::<Option<String>>(db)
        .await?;

    Ok(join_non_empty_pages(ocr_pages))
}

fn join_non_empty_pages(pages: Vec<Option<String>>) -> String {
    pages
        .into_iter()
        .filter_map(|page| page)
        .map(|page| page.trim().to_string())
        .filter(|page| !page.is_empty())
        .collect::<Vec<String>>()
        .join("\n\n")
}

async fn load_classifier_blocks(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
) -> JobResult<Vec<ClassifierBlock>> {
    Ok(classifier_blocks::table
        .filter(classifier_blocks::enabled.eq(true))
        .order((classifier_blocks::order.asc(), classifier_blocks::id.asc()))
        .select(ClassifierBlock::as_select())
        .load::<ClassifierBlock>(db)
        .await?)
}

fn find_first_match<'a>(
    document: &'a DocumentView,
    document_text: &'a str,
    patterns: &'a [ClassifierPattern],
) -> JobResult<Option<PatternMatch<'a>>> {

    // Allow empty patterns to match by default, so that we can apply global child rules at any point during the flow.
    if patterns.len() == 0 {
        return Ok(Some(PatternMatch::Metadata));
    }

    for pattern in patterns {
        let pattern_match = does_document_match_pattern(document, document_text, pattern)?;
        if !matches!(pattern_match, PatternMatch::None) {
            return Ok(Some(pattern_match));
        }
    }

    Ok(None)
}

fn apply_match_actions(
    computed_actions: &mut HashMap<String, String>,
    actions: &HashMap<String, String>,
) {
    for (key, value) in actions {
        tracing::debug!(
            metadata_key = key,
            metadata_value = value,
            "classification: applying match action"
        );
        computed_actions.insert(key.clone(), value.clone());
    }
}

fn apply_child_rules(
    document: &DocumentView,
    document_text: &str,
    computed_actions: &mut HashMap<String, String>,
    child_rules: &Vec<ClassifierChildRule>,
) -> JobResult<()> {
    for rule in child_rules {
        // Skip non-matching rules
        let matched = does_document_match_pattern(document, document_text, &rule.pattern)?;
        if matches!(matched, PatternMatch::None) {
            continue;
        }

        let mut snippets: HashMap<u32, String> = HashMap::new();

        // We now will extract match groups and apply modifiers.
        if let PatternMatch::Text(captures) = matched {
            // - Extract captured groups into the snippets.
            for (i, cap) in captures.iter().enumerate() {
                if i > 0
                    && let Some(cap) = cap
                {
                    snippets.insert(i as u32, cap.as_str().to_string());
                }
            }

            // - For each modifier in the rule, apply the modifier to the snippets.
            if let Some(modifiers) = &rule.modifiers {
                for modifier in modifiers {
                    apply_modifier(&mut snippets, &modifier, computed_actions);
                }
            }
        }

        // Now, collect all of the match actions, and apply them to the computed metadata.
        // But in this phase, we will need to apply replacements to the values, based on the snippets.
        for (key, value) in &rule.actions {
            let replaced_value = apply_replacements(value, &snippets);
            tracing::debug!(
                metadata_key = key,
                metadata_value = value,
                "classification: applying child rule action"
            );
            computed_actions.insert(key.clone(), replaced_value);
        }
    }

    Ok(())
}

fn apply_replacements(value: &str, snippets: &HashMap<u32, String>) -> String {
    let reg = regex::Regex::new(r"\\(\d+)").unwrap();
    reg.replace_all(value, |caps: &Captures| {
        let index = caps[1].parse::<u32>().unwrap();
        snippets.get(&index).cloned().unwrap_or_default()
    })
    .to_string()
}

fn does_document_match_pattern<'a>(
    document: &'a DocumentView,
    document_text: &'a str,
    pattern: &'a ClassifierPattern,
) -> JobResult<PatternMatch<'a>> {
    // If the pattern has metadata, check if the document metadata contains all of the key-value pairs in the pattern metadata.
    if let Some(pattern_metadata) = &pattern.metadata {
        for (key, value) in pattern_metadata {
            tracing::debug!(
                document_id = document.id,
                metadata_key = key,
                metadata_value = value,
                "classification: testing metadata"
            );
            match document.metadata.get(key) {
                Some(document_value) if document_value == value => (),
                _ => return Ok(PatternMatch::None),
            }
        }
    }

    // If the pattern has text, check if the document content text contains the pattern text.
    if let Some(pattern_text) = &pattern.text {
        tracing::debug!(
            document_id = document.id,
            pattern_text = pattern.text,
            "classification: testing pattern"
        );
        // Convert the pattern text to a regex pattern.
        // Test if the document text matches the regex pattern.
        let reg = Regex::new(pattern_text)?;
        let cap = reg.captures(document_text);
        return match cap {
            None => Ok(PatternMatch::None),
            Some(captures) => Ok(PatternMatch::Text(captures)),
        };

        // Otherwise, if the pattern is empty, we allow it to be a match.
    } else if pattern.metadata.is_none() {
        return Ok(PatternMatch::Metadata);
    }

    // The document matched
    Ok(PatternMatch::Metadata)
}

fn apply_modifier(
    snippets: &mut HashMap<u32, String>,
    modifier: &ClassifierModifier,
    computed_actions: &mut HashMap<String, String>,
) {
    match modifier {
        ClassifierModifier::Metadata { to, slug } => {
            if let Some(value) = computed_actions.get(slug) {
                snippets.insert(*to, value.clone());
            }
        }
        ClassifierModifier::MonthNumber { from, to } => {
            let value = apply_replacements(from, snippets);
            if let Some(value) = mod_month_number(&value) {
                snippets.insert(*to, value.clone());
            }
        }
        ClassifierModifier::MonthEnd { from, to } => {
            let value = apply_replacements(from, snippets);
            if let Some(value) = mod_month_end(&value, None).ok() {
                snippets.insert(*to, value.clone());
            }
        }
        ClassifierModifier::MonthStart { from, to } => {
            let value = apply_replacements(from, snippets);
            if let Some(value) = mod_month_start(&value, None).ok() {
                snippets.insert(*to, value.clone());
            }
        }
        ClassifierModifier::NextDay { from, to } => {
            let value = apply_replacements(from, snippets);
            if let Some(value) = mod_next_day(&value, None).ok() {
                snippets.insert(*to, value.clone());
            }
        }
        ClassifierModifier::PrevDay { from, to } => {
            let value = apply_replacements(from, snippets);
            if let Some(value) = mod_prev_day(&value, None).ok() {
                snippets.insert(*to, value.clone());
            }
        }
        ClassifierModifier::NextMonth { from, to } => {
            let value = apply_replacements(from, snippets);
            if let Some(value) = mod_next_month(&value, None).ok() {
                snippets.insert(*to, value.clone());
            }
        }
        ClassifierModifier::PrevMonth { from, to } => {
            let value = apply_replacements(from, snippets);
            if let Some(value) = mod_prev_month(&value, None).ok() {
                snippets.insert(*to, value.clone());
            }
        }
        ClassifierModifier::TaxYear { from, to } => {
            let value = apply_replacements(from, snippets);
            if let Some(value) = mod_tax_year(&value).ok() {
                snippets.insert(*to, value.clone());
            }
        }
        ClassifierModifier::Currency { from, to } => {
            let value = apply_replacements(from, snippets);
            snippets.insert(*to, mod_currency(&value));
        }
        ClassifierModifier::Sprintf { from, to, format } => {
            let value = apply_replacements(from, snippets);
            snippets.insert(*to, mod_sprintf(&value, format));
        }
        ClassifierModifier::Replace { from, to } => {
            let value = apply_replacements(from, snippets);
            snippets.insert(*to, value);
        }
        ClassifierModifier::AlnumSanitize { from, to } => {
            let value = apply_replacements(from, snippets);
            snippets.insert(*to, mod_alnum_sanitize(&value));
        }
        ClassifierModifier::DateFormat { from, to, format } => {
            let value = apply_replacements(from, snippets);
            if let Some(value) = mod_date_format(&value, Some(format)).ok() {
                snippets.insert(*to, value.clone());
            }
        }
        ClassifierModifier::Add { from, to } => {
            if let Some(value) = mod_add(snippets, *from, *to) {
                snippets.insert(*to, value);
            }
        }
        ClassifierModifier::Sub { from, to } => {
            if let Some(value) = mod_sub(snippets, *from, *to) {
                snippets.insert(*to, value);
            }
        }
        ClassifierModifier::Mul { from, to } => {
            if let Some(value) = mod_mul(snippets, *from, *to) {
                snippets.insert(*to, value);
            }
        }
        ClassifierModifier::Div { from, to } => {
            if let Some(value) = mod_div(snippets, *from, *to) {
                snippets.insert(*to, value);
            }
        }
    }
}

fn snippet_number(snippets: &HashMap<u32, String>, index: u32) -> Option<f64> {
    let value = snippets.get(&index)?;
    let normalized = value.trim().replace([',', '$'], "");
    normalized.parse::<f64>().ok()
}

fn format_number(value: f64) -> String {
    if value.fract().abs() < f64::EPSILON {
        return (value as i64).to_string();
    }

    let mut formatted = format!("{value:.10}");
    while formatted.contains('.') && formatted.ends_with('0') {
        formatted.pop();
    }
    if formatted.ends_with('.') {
        formatted.pop();
    }
    formatted
}

fn mod_add(snippets: &HashMap<u32, String>, from: u32, to: u32) -> Option<String> {
    Some(format_number(
        snippet_number(snippets, to)? + snippet_number(snippets, from)?,
    ))
}

fn mod_sub(snippets: &HashMap<u32, String>, from: u32, to: u32) -> Option<String> {
    Some(format_number(
        snippet_number(snippets, to)? - snippet_number(snippets, from)?,
    ))
}

fn mod_mul(snippets: &HashMap<u32, String>, from: u32, to: u32) -> Option<String> {
    Some(format_number(
        snippet_number(snippets, to)? * snippet_number(snippets, from)?,
    ))
}

fn mod_div(snippets: &HashMap<u32, String>, from: u32, to: u32) -> Option<String> {
    let denominator = snippet_number(snippets, from)?;
    if denominator.abs() < f64::EPSILON {
        return None;
    }

    Some(format_number(snippet_number(snippets, to)? / denominator))
}

fn mod_month_number(value: &str) -> Option<String> {
    // Normalize: remove spaces and capitalize first letter, lowercase rest
    let name = value.replace(' ', "").to_lowercase();

    // Month name tables (like Ruby's Date::MONTHNAMES / ABBR_MONTHNAMES)
    const MONTHS: [&str; 12] = [
        "january",
        "february",
        "march",
        "april",
        "may",
        "june",
        "july",
        "august",
        "september",
        "october",
        "november",
        "december",
    ];

    const ABBR_MONTHS: [&str; 12] = [
        "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
    ];

    // Find index (like Ruby's `.index`)
    let number = MONTHS
        .iter()
        .position(|&m| m == name)
        .or_else(|| ABBR_MONTHS.iter().position(|&m| m == name));

    // Format like '%02d'
    let result = number.map(|n| format!("{:02}", n + 1));
    tracing::debug!(value, result = &result, "classification: modifier: month_number");
    result
}

fn mod_sprintf(value: &str, fmt: &str) -> String {
    tracing::debug!(value, fmt, "classification: modifier: sprintf");
    let mut v = value;
    let re = Regex::new(r"^0+([1-9])").unwrap();
    if let Some(caps) = re.captures(value) {
        if let Some(m) = caps.get(1) {
            v = m.as_str();
        }
    }

    sprintf!(fmt, v).unwrap().replace(" ", "0")
}

fn mod_month_end(value: &str, fmt: Option<&str>) -> Result<String, chrono::ParseError> {
    // Equivalent to Date.parse(value)
    let d = NaiveDate::parse_from_str(value, "%Y-%m-%d")?;

    // Equivalent to (Date.parse(value) << -1)
    // In Ruby, << -1 means advance by 1 month
    let (year, month) = if d.month() == 12 {
        (d.year() + 1, 1)
    } else {
        (d.year(), d.month() + 1)
    };

    // Equivalent to Date.new(d.year, d.month, 1) - 1
    let first_of_next_month = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
    let last_of_original_shifted_month = first_of_next_month - Duration::days(1);

    // Equivalent to .strftime(args[0] || '%Y-%m-%d')
    let format_str = fmt.unwrap_or("%Y-%m-%d");
    Ok(last_of_original_shifted_month
        .format(format_str)
        .to_string())
}

fn mod_month_start(value: &str, fmt: Option<&str>) -> Result<String, chrono::ParseError> {
    let d = NaiveDate::parse_from_str(value, "%Y-%m-%d")?;
    let first_of_month = NaiveDate::from_ymd_opt(d.year(), d.month(), 1).unwrap();
    Ok(first_of_month.format(fmt.unwrap_or("%Y-%m-%d")).to_string())
}

fn mod_next_day(value: &str, fmt: Option<&str>) -> Result<String, Box<dyn std::error::Error>> {
    let mut parts = value.splitn(2, '|').collect::<Vec<_>>();
    parts.reverse();

    let (date_str, days) = match parts.as_slice() {
        [date] => (*date, 1),
        [date, days_str] => (*date, days_str.parse::<i64>()?),
        _ => return Err("invalid input".into()),
    };

    // (Date.parse(value) + days)
    let d = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")?;
    let shifted = d + Duration::days(days);

    // .strftime(args[0] || '%Y-%m-%d')
    Ok(shifted.format(fmt.unwrap_or("%Y-%m-%d")).to_string())
}

fn mod_prev_day(value: &str, fmt: Option<&str>) -> Result<String, Box<dyn std::error::Error>> {
    // Equivalent to: value, days = value.split('|', 2).reverse
    let mut parts = value.splitn(2, '|').collect::<Vec<_>>();
    parts.reverse();

    let (date_str, days) = match parts.as_slice() {
        [date] => (*date, 1),
        [date, days_str] => (*date, days_str.parse::<i64>()?),
        _ => return Err("invalid input".into()),
    };

    // Date.parse(value)
    let d = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")?;

    // (Date.parse(value) - days)
    let shifted = d - Duration::days(days);

    // .strftime(args[0] || '%Y-%m-%d')
    Ok(shifted.format(fmt.unwrap_or("%Y-%m-%d")).to_string())
}

fn mod_next_month(value: &str, fmt: Option<&str>) -> Result<String, Box<dyn std::error::Error>> {
    // Equivalent to: value, months = value.split('|', 2).reverse
    let mut parts = value.splitn(2, '|').collect::<Vec<_>>();
    parts.reverse();

    let (date_str, months) = match parts.as_slice() {
        [date] => (*date, 1),
        [date, months_str] => (*date, months_str.parse::<i32>()?),
        _ => return Err("invalid input".into()),
    };

    let d = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")?;

    // Equivalent to Date.parse(value) >> months
    let shifted = add_months(d, months);

    Ok(shifted.format(fmt.unwrap_or("%Y-%m-%d")).to_string())
}

fn mod_prev_month(value: &str, fmt: Option<&str>) -> Result<String, Box<dyn std::error::Error>> {
    // Equivalent to: value, months = value.split('|', 2).reverse
    let mut parts = value.splitn(2, '|').collect::<Vec<_>>();
    parts.reverse();

    let (date_str, months) = match parts.as_slice() {
        [date] => (*date, 1),
        [date, months_str] => (*date, months_str.parse::<i32>()?),
        _ => return Err("invalid input".into()),
    };

    let d = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")?;

    // Equivalent to: Date.parse(value) << months
    let shifted = add_months(d, -months);

    Ok(shifted.format(fmt.unwrap_or("%Y-%m-%d")).to_string())
}

fn mod_tax_year(value: &str) -> Result<String, chrono::ParseError> {
    // Date.parse(value)
    let d = NaiveDate::parse_from_str(value, "%Y-%m-%d")?;

    // (Date.parse(value) << -1)  => add 1 month
    let shifted = add_months(d, 1);

    // Date.new(d.year, d.month, 1).year.to_s
    Ok(shifted.year().to_string())
}

fn mod_currency(value: &str) -> String {
    let re = Regex::new(r"^\$?0*([1-9])").unwrap();
    let result = re.replace(value, "$1");
    result.replace(',', "")
}

fn mod_date_format(value: &str, fmt: Option<&str>) -> Result<String, chrono::ParseError> {
    let d = NaiveDate::parse_from_str(value, "%Y-%m-%d")?;
    Ok(d.format(fmt.unwrap_or("%Y-%m-%d")).to_string())
}

fn mod_alnum_sanitize(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    let mut last_was_space = false;

    for c in value.chars() {
        if c.is_ascii_alphanumeric() {
            result.push(c);
            last_was_space = false;
        } else if c.is_whitespace() {
            if !last_was_space {
                result.push(' ');
                last_was_space = true;
            }
        }
        // else: drop non-alnum chars
    }

    result.trim().to_string()
}

fn add_months(date: NaiveDate, months: i32) -> NaiveDate {
    let year = date.year();
    let month0 = date.month0() as i32; // 0-based
    let total = year * 12 + month0 + months;

    let new_year = total.div_euclid(12);
    let new_month0 = total.rem_euclid(12);
    let new_month = (new_month0 + 1) as u32;

    let last_day = last_day_of_month(new_year, new_month);
    let day = date.day().min(last_day);

    NaiveDate::from_ymd_opt(new_year, new_month, day).unwrap()
}

fn last_day_of_month(year: i32, month: u32) -> u32 {
    for day in (28..=31).rev() {
        if NaiveDate::from_ymd_opt(year, month, day).is_some() {
            return day;
        }
    }
    unreachable!()
}
