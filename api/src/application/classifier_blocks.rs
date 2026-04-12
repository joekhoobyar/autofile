use std::collections::HashMap;
use std::sync::Arc;

use apalis::prelude::*;
use bb8::PooledConnection;
use diesel::prelude::*;
use diesel_async::AsyncPgConnection;
use diesel_async::RunQueryDsl;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use regex::{Regex, Captures};
use sprintf::sprintf;
use chrono::{Datelike, Duration, NaiveDate};

use crate::application::documents::get_document_view;
use crate::domain::classifier_blocks::ClassifierModifier;
use crate::domain::classifier_blocks::{ClassifierBlock, ClassifierPattern, ClassifierChildRule};
use crate::domain::documents::DocumentView;
use crate::schema::{
    classifier_blocks, document_file_ocr_pages, document_file_pages, document_files,
};
use crate::shared::app_state::AppState;
use crate::shared::util::JobResult;

#[derive(Debug)]
pub enum PatternMatch<'a> {
    None,
    Text(Captures<'a>),
    Metadata,
}

pub async fn classify_document(document_id: i64, state: Data<Arc<AppState>>) -> Result<(), Error> {
    classify_document_inner(document_id, state)
        .await
        .map_err(Into::into)
}

async fn classify_document_inner(document_id: i64, state: Data<Arc<AppState>>) -> JobResult<()> {
    tracing::info!(document_id, "classifying document");

    let mut db = state.db_pool.get().await?;

    // Load the document view from the database
    let document_view = get_document_view(&mut db, document_id).await?;

    // Load the document text, trying content text first, then falling back to OCR.
    // Join all of the pages together into a single string.
    let document_text = load_document_text(&mut db, document_id).await?;

    // Load all of the classifier blocks from the database, ordered by their "order" field.
    let classifier_blocks = load_classifier_blocks(&mut db).await?;

    let mut computed_metadata = HashMap::new();

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
            apply_match_actions(&mut computed_metadata, &rules.match_actions);
            apply_child_rules(&document_view, &document_text, &mut computed_metadata, &rules.child_rules)?;
            break
        }
    }

    tracing::info!(
        document_id,
        ?computed_metadata,
        "computed classifier metadata"
    );

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
    for pattern in patterns {
        let pattern_match = does_document_match_pattern(document, document_text, pattern)?;
        if !matches!(pattern_match, PatternMatch::None) {
            return Ok(Some(pattern_match));
        }
    }

    Ok(None)
}

fn apply_match_actions(
    computed_metadata: &mut HashMap<String, String>,
    actions: &HashMap<String, String>,
) {
    for (key, value) in actions {
        computed_metadata.insert(key.clone(), value.clone());
    }
}

fn apply_child_rules(
    document: &DocumentView,
    document_text: &str,
    computed_metadata: &mut HashMap<String, String>,
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

            // - Extract captured groups into the snippets Vec.
            for (i, cap) in captures.iter().enumerate() {
                if i > 0 && let Some(cap) = cap {
                    snippets.insert(i as u32, cap.as_str().to_string());
                }
            }

            // - For each modifier in the rule, apply the modifier to the Vec.
            if let Some(modifiers) = &rule.modifiers {
                for modifier in modifiers {
                    apply_modifier(&mut snippets, &modifier, computed_metadata);
                }
            }
        }

        // Finally, collect all of the match actions, and apply them to the computed metadata.
        // But in this phase, we will need to apply replacements to the values, based on the Vec.
        for (key, value) in &rule.actions {
            let replaced_value = apply_replacements(value, &snippets);
            computed_metadata.insert(key.clone(), replaced_value);
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
            match document.metadata.get(key) {
                Some(document_value) if document_value == value => (),
                _ => return Ok(PatternMatch::None),
            }
        }
    }

    // If the pattern has text, check if the document content text contains the pattern text.
    if let Some(pattern_text) = &pattern.text {
        // Convert the pattern text to a regex pattern.
        // Test if the document text matches the regex pattern.
        let reg = Regex::new(pattern_text)?;
        let cap = reg.captures(document_text);
        return match cap {
            None => Ok(PatternMatch::None),
            Some(captures) => Ok(PatternMatch::Text(captures)),
        };
    } else if pattern.metadata.is_none() {
        return Ok(PatternMatch::None);
    }

    // The document matched
    Ok(PatternMatch::Metadata)
}

fn apply_modifier(
    snippets: &mut HashMap<u32, String>,
    modifier: &ClassifierModifier,
    computed_metadata: &mut HashMap<String, String>,
) {
    match modifier {
        ClassifierModifier::Metadata { to, slug } => {
            if let Some(value) = computed_metadata.get(slug) {
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
        },
        ClassifierModifier::MonthStart { from, to } => {
            let value = apply_replacements(from, snippets);
            if let Some(value) = mod_month_start(&value, None).ok() {
                snippets.insert(*to, value.clone());
            }
        },
        ClassifierModifier::NextDay { from, to } => todo!(),
        ClassifierModifier::PrevDay { from, to } => todo!(),
        ClassifierModifier::NextMonth { from, to } => todo!(),
        ClassifierModifier::PrevMonth { from, to } => todo!(),
        ClassifierModifier::TaxYear { from, to } => todo!(),
        ClassifierModifier::Currency { from, to } => todo!(),
        ClassifierModifier::Sprintf { from, to, format } => {
            let value = apply_replacements(from, snippets);
            snippets.insert(*to, mod_sprintf(&value, format));
        }
        ClassifierModifier::Replace { from, to } => todo!(),
        ClassifierModifier::AlnumSanitize { from, to } => todo!(),
        ClassifierModifier::DateFormat { from, to, format } => todo!(),
        ClassifierModifier::Add { from, to } => todo!(),
        ClassifierModifier::Sub { from, to } => todo!(),
        ClassifierModifier::Mul { from, to } => todo!(),
        ClassifierModifier::Div { from, to } => todo!(),
    }
}


fn mod_month_number(value: &str) -> Option<String> {
    // Normalize: remove spaces and capitalize first letter, lowercase rest
    let mut name = value.replace(' ', "").to_lowercase();
    if let Some(first) = name.get_mut(0..1) {
        first.make_ascii_uppercase();
    }

    // Month name tables (like Ruby's Date::MONTHNAMES / ABBR_MONTHNAMES)
    const MONTHS: [&str; 13] = [
        "", "January", "February", "March", "April", "May", "June",
        "July", "August", "September", "October", "November", "December",
    ];

    const ABBR_MONTHS: [&str; 13] = [
        "", "Jan", "Feb", "Mar", "Apr", "May", "Jun",
        "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];

    // Find index (like Ruby's `.index`)
    let number = MONTHS.iter().position(|&m| m == name)
        .or_else(|| ABBR_MONTHS.iter().position(|&m| m == name));

    // Format like '%02d'
    number.map(|n| format!("{:02}", n))
}


fn mod_sprintf(value: &str, fmt: &str) -> String {
    let mut v= value;
    let re = Regex::new(r"^0+([1-9])").unwrap();
    if let Some(caps) = re.captures(value) {
        if let Some(m) = caps.get(1) {
            v = m.as_str();
        }
    }

    sprintf!(fmt, v).unwrap()
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
    Ok(last_of_original_shifted_month.format(format_str).to_string())
}

fn mod_month_start(value: &str, fmt: Option<&str>) -> Result<String, chrono::ParseError> {
    // Equivalent to Date.parse(value)
    let d = NaiveDate::parse_from_str(value, "%Y-%m-%d")?;

    // Equivalent to Date.new(d.year, d.month, 1)
    let first_of_month = NaiveDate::from_ymd_opt(d.year(), d.month(), 1).unwrap();

    // Equivalent to .strftime(args[0] || '%Y-%m-%d')
    let format_str = fmt.unwrap_or("%Y-%m-%d");
    Ok(first_of_month.format(format_str).to_string())
}
