use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use apalis::prelude::*;
use bb8::PooledConnection;
use chrono::{Datelike, Duration, NaiveDate, Utc};
use diesel::prelude::*;
use diesel::sql_types::{BigInt, Integer};
use diesel::upsert::excluded;
use diesel_async::AsyncConnection;
use diesel_async::AsyncPgConnection;
use diesel_async::RunQueryDsl;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use regex::{Captures, Regex, RegexBuilder};
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
use crate::shared::util::{ApiError, JobResult, diesel_to_http};

#[derive(Debug)]
pub struct UpdateClassifierBlockInput {
    pub name: Option<String>,
    pub description: Option<String>,
    pub enabled: Option<bool>,
    pub rules: Option<crate::domain::classifier_blocks::ClassifierRules>,
}

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
    classify_document_inner_without_enqueue(document_id, user_id, state.clone()).await?;

    enqueue_document_index_document_updates(document_id, (*state).clone()).await?;

    Ok(())
}

pub async fn classify_document_inner_without_enqueue(
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

    let computed_actions = compute_classification_actions(
        document_id,
        &document_view,
        &document_text,
        &classifier_blocks,
    )?;

    // Finally, we will have a set of computed actions that we want to apply to the document.
    persist_computed_actions(&mut db, document_id, user_id, computed_actions).await?;

    Ok(())
}

pub fn compute_classification_actions(
    document_id: i64,
    document_view: &DocumentView,
    document_text: &str,
    classifier_blocks: &[ClassifierBlock],
) -> JobResult<HashMap<String, String>> {
    let mut computed_actions = HashMap::new();

    // Iterate over each classifier block and pull out it's ClassifierRules.
    // Try to match the document against the ClassifierRules, starting with the match_patterns.
    // call does_document_match_pattern for each match_pattern
    for classifier_block in classifier_blocks {
        let rules = &classifier_block.rules.0;

        let pattern_match = find_first_match(
            document_view,
            document_text,
            &computed_actions,
            &rules.match_patterns,
        )?;

        // If any of the match_patterns match, then we can apply the match_actions to the document,
        // and then move on to the child_rules application.
        if pattern_match.is_some() {
            tracing::info!(
                document_id,
                classifier_block_id = classifier_block.id,
                "classification: block matched"
            );
            apply_match_actions(&mut computed_actions, &rules.match_actions);
            apply_child_rules(
                classifier_block.id,
                document_view,
                document_text,
                &mut computed_actions,
                &rules.child_rules,
            )?;

            if !rules.continue_after_match {
                break;
            }
        }
    }

    Ok(computed_actions)
}

pub async fn persist_computed_actions(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    document_id: i64,
    user_id: i64,
    computed_actions: HashMap<String, String>,
) -> JobResult<()> {
    // Iterate over all of the computed actions.
    tracing::info!(
        document_id,
        ?computed_actions,
        "classification: computed actions"
    );
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
                    .first::<i64>(db)
                    .await?;
                document_type_id = Some(doctype);
            }
            "_suggested_filename" => {
                tracing::info!(
                    document_id,
                    title = value,
                    "classification: suggested title"
                );
                title = Some(value);
            }
            "_suggested_tags" => {
                tracing::info!(document_id, tags = value, "classification: suggested tags");
                let slugs = parse_slug_list(&value);
                apply_suggested_tags(db, document_id, user_id, &slugs).await?;
            }
            "_suggested_cabinets" => {
                tracing::info!(
                    document_id,
                    cabinets = value,
                    "classification: suggested cabinet"
                );
                let slugs = parse_slug_list(&value);
                apply_suggested_cabinets(db, document_id, user_id, &slugs).await?;
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
            db,
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
            .load::<(String, i64)>(db)
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
            document_metadatas_upsert(user_id, db, document_id, metadata_input).await?;
        }
    }

    Ok(())
}

pub async fn create_classifier_block(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    user_id: i64,
    name: String,
    description: Option<String>,
    enabled: bool,
    rules: crate::domain::classifier_blocks::ClassifierRules,
) -> Result<ClassifierBlock, ApiError> {
    db.transaction::<_, diesel::result::Error, _>(move |conn| {
        Box::pin(async move {
            diesel::sql_query("LOCK TABLE classifier_blocks IN EXCLUSIVE MODE")
                .execute(conn)
                .await?;

            let next_order = classifier_blocks::table
                .select(diesel::dsl::max(classifier_blocks::order))
                .get_result::<Option<i32>>(conn)
                .await?
                .unwrap_or(0)
                + 1;

            diesel::insert_into(classifier_blocks::table)
                .values((
                    classifier_blocks::name.eq(name),
                    classifier_blocks::description.eq(description),
                    classifier_blocks::enabled.eq(enabled),
                    classifier_blocks::order.eq(next_order),
                    classifier_blocks::rules.eq(diesel_json::Json(rules)),
                    classifier_blocks::created_by.eq(user_id),
                    classifier_blocks::updated_by.eq(user_id),
                ))
                .returning(ClassifierBlock::as_returning())
                .get_result(conn)
                .await
        })
    })
    .await
    .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to create classifier_block"))
}

pub async fn update_classifier_block(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    user_id: i64,
    id: i64,
    input: UpdateClassifierBlockInput,
) -> Result<ClassifierBlock, ApiError> {
    diesel::update(classifier_blocks::table.filter(classifier_blocks::id.eq(id)))
        .set((
            input.name.map(|value| classifier_blocks::name.eq(value)),
            input
                .description
                .map(|value| classifier_blocks::description.eq(value)),
            input
                .enabled
                .map(|value| classifier_blocks::enabled.eq(value)),
            input
                .rules
                .map(|value| classifier_blocks::rules.eq(diesel_json::Json(value))),
            classifier_blocks::updated_at.eq(diesel::dsl::now),
            classifier_blocks::updated_by.eq(user_id),
        ))
        .returning(ClassifierBlock::as_returning())
        .get_result(db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to update classifier_block"))
}

pub async fn delete_classifier_block(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    id: i64,
) -> Result<(), ApiError> {
    db.transaction::<_, diesel::result::Error, _>(move |conn| {
        Box::pin(async move {
            diesel::sql_query("LOCK TABLE classifier_blocks IN EXCLUSIVE MODE")
                .execute(conn)
                .await?;

            let deleted_order = classifier_blocks::table
                .filter(classifier_blocks::id.eq(id))
                .select(classifier_blocks::order)
                .first::<i32>(conn)
                .await?;

            diesel::delete(classifier_blocks::table.filter(classifier_blocks::id.eq(id)))
                .execute(conn)
                .await?;

            diesel::update(
                classifier_blocks::table.filter(classifier_blocks::order.gt(deleted_order)),
            )
            .set(classifier_blocks::order.eq(classifier_blocks::order - 1))
            .execute(conn)
            .await?;

            Ok(())
        })
    })
    .await
    .map_err(|e| {
        if matches!(e, diesel::result::Error::NotFound) {
            ApiError::not_found("Classifier block not found")
        } else {
            ApiError::new(diesel_to_http(e), "Failed to delete classifier_block")
        }
    })
}

#[derive(Debug)]
enum ReorderClassifierBlockError {
    Api(ApiError),
    Diesel(diesel::result::Error),
}

impl From<ApiError> for ReorderClassifierBlockError {
    fn from(value: ApiError) -> Self {
        Self::Api(value)
    }
}

impl From<diesel::result::Error> for ReorderClassifierBlockError {
    fn from(value: diesel::result::Error) -> Self {
        Self::Diesel(value)
    }
}

pub async fn reorder_classifier_block(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    user_id: i64,
    id: i64,
    target_order: i32,
) -> Result<ClassifierBlock, ApiError> {
    db.transaction::<_, ReorderClassifierBlockError, _>(move |conn| {
        Box::pin(async move {
            diesel::sql_query("LOCK TABLE classifier_blocks IN EXCLUSIVE MODE")
                .execute(conn)
                .await
                .map_err(|e| {
                    ApiError::new(diesel_to_http(e), "Failed to lock classifier_blocks")
                })?;

            let current_order = classifier_blocks::table
                .filter(classifier_blocks::id.eq(id))
                .select(classifier_blocks::order)
                .first::<i32>(conn)
                .await
                .map_err(|e| {
                    if matches!(e, diesel::result::Error::NotFound) {
                        ApiError::not_found("Classifier block not found")
                    } else {
                        ApiError::new(diesel_to_http(e), "Failed to fetch classifier_block")
                    }
                })?;

            let max_order = classifier_blocks::table
                .select(diesel::dsl::max(classifier_blocks::order))
                .get_result::<Option<i32>>(conn)
                .await
                .map_err(|e| {
                    ApiError::new(diesel_to_http(e), "Failed to fetch classifier_block order")
                })?
                .unwrap_or(0);

            if !(1..=max_order).contains(&target_order) {
                return Err(ApiError::bad_request(
                    "Classifier block order must be within the current list bounds",
                )
                .into());
            }

            if target_order == current_order {
                return classifier_blocks::table
                    .find(id)
                    .select(ClassifierBlock::as_select())
                    .first::<ClassifierBlock>(conn)
                    .await
                    .map_err(|e| {
                        ApiError::new(diesel_to_http(e), "Failed to fetch classifier_block")
                    })
                    .map_err(Into::into);
            }

            diesel::sql_query("SET CONSTRAINTS classifier_blocks_order_key DEFERRED")
                .execute(conn)
                .await
                .map_err(|e| {
                    ApiError::new(
                        diesel_to_http(e),
                        "Failed to prepare classifier_block reorder",
                    )
                })?;

            diesel::sql_query(
                r#"
                UPDATE classifier_blocks
                SET
                    "order" = CASE
                        WHEN id = $1 THEN $2
                        WHEN $2 < $3 AND "order" >= $2 AND "order" < $3 THEN "order" + 1
                        WHEN $2 > $3 AND "order" > $3 AND "order" <= $2 THEN "order" - 1
                        ELSE "order"
                    END,
                    updated_at = CASE
                        WHEN id = $1 THEN NOW()
                        ELSE updated_at
                    END,
                    updated_by = CASE
                        WHEN id = $1 THEN $4
                        ELSE updated_by
                    END
                WHERE id = $1
                   OR "order" BETWEEN LEAST($2, $3) AND GREATEST($2, $3)
                "#,
            )
            .bind::<BigInt, _>(id)
            .bind::<Integer, _>(target_order)
            .bind::<Integer, _>(current_order)
            .bind::<BigInt, _>(user_id)
            .execute(conn)
            .await
            .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to reorder classifier_block"))?;

            classifier_blocks::table
                .find(id)
                .select(ClassifierBlock::as_select())
                .first::<ClassifierBlock>(conn)
                .await
                .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to fetch classifier_block"))
                .map_err(Into::into)
        })
    })
    .await
    .map_err(|e| match e {
        ReorderClassifierBlockError::Api(err) => err,
        ReorderClassifierBlockError::Diesel(err) => {
            ApiError::new(diesel_to_http(err), "Failed to reorder classifier_block")
        }
    })
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
            tracing::warn!(
                document_id,
                slug,
                "classification: suggested cabinet not found"
            );
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

pub async fn load_document_text(
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

pub async fn load_classifier_blocks(
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
    computed_actions: &HashMap<String, String>,
    patterns: &'a [ClassifierPattern],
) -> JobResult<Option<PatternMatch<'a>>> {
    // Allow empty patterns to match by default, so that we can apply global child rules at any point during the flow.
    if patterns.len() == 0 {
        return Ok(Some(PatternMatch::Metadata));
    }

    for pattern in patterns {
        let pattern_match =
            does_document_match_pattern(document, document_text, computed_actions, pattern)?;
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
    classifier_block_id: i64,
    document: &DocumentView,
    document_text: &str,
    computed_actions: &mut HashMap<String, String>,
    child_rules: &Vec<ClassifierChildRule>,
) -> JobResult<()> {
    for rule in child_rules {
        // Skip non-matching rules
        let matched =
            does_document_match_pattern(document, document_text, computed_actions, &rule.pattern)?;
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
                    match apply_modifier(&snippets, modifier, computed_actions) {
                        Ok(Some((to, value))) => {
                            snippets.insert(to, value);
                        }
                        Ok(None) => {}
                        Err(error) => {
                            tracing::warn!(
                                document_id = document.id,
                                classifier_block_id,
                                modifier_type = modifier_kind(modifier),
                                ?modifier,
                                error,
                                "classification: modifier failed"
                            );
                        }
                    }
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
    computed_actions: &HashMap<String, String>,
    pattern: &'a ClassifierPattern,
) -> JobResult<PatternMatch<'a>> {
    // If the pattern has metadata, check if the document metadata contains all of the key-value pairs in the pattern metadata.
    if let Some(pattern_metadata) = &pattern.metadata {
        for (key, value) in pattern_metadata {
            if let Some(computed_value) = computed_actions.get(key) {
                tracing::debug!(
                    document_id = document.id,
                    metadata_key = key,
                    metadata_expected_value = value,
                    metadata_actual_value = computed_value,
                    metadata_source = "computed_actions",
                    "classification: testing metadata"
                );

                if computed_value != value {
                    return Ok(PatternMatch::None);
                }

                continue;
            }

            match document.metadata.get(key) {
                Some(document_value) => {
                    tracing::debug!(
                        document_id = document.id,
                        metadata_key = key,
                        metadata_expected_value = value,
                        metadata_actual_value = document_value,
                        metadata_source = "document",
                        "classification: testing metadata"
                    );

                    if document_value != value {
                        return Ok(PatternMatch::None);
                    }
                }
                None => {
                    tracing::debug!(
                        document_id = document.id,
                        metadata_key = key,
                        metadata_expected_value = value,
                        metadata_source = "document",
                        "classification: testing metadata"
                    );

                    return Ok(PatternMatch::None);
                }
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
        let reg = RegexBuilder::new(pattern_text)
            .case_insensitive(true)
            .multi_line(true)
            .build()?;
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

fn modifier_kind(modifier: &ClassifierModifier) -> &'static str {
    match modifier {
        ClassifierModifier::Metadata { .. } => "metadata",
        ClassifierModifier::MonthNumber { .. } => "month_number",
        ClassifierModifier::MonthEnd { .. } => "month_end",
        ClassifierModifier::MonthStart { .. } => "month_start",
        ClassifierModifier::NextDay { .. } => "next_day",
        ClassifierModifier::PrevDay { .. } => "prev_day",
        ClassifierModifier::NextMonth { .. } => "next_month",
        ClassifierModifier::PrevMonth { .. } => "prev_month",
        ClassifierModifier::TaxYear { .. } => "tax_year",
        ClassifierModifier::Currency { .. } => "currency",
        ClassifierModifier::Sprintf { .. } => "sprintf",
        ClassifierModifier::ZeroPad { .. } => "zero_pad",
        ClassifierModifier::Replace { .. } => "replace",
        ClassifierModifier::AlnumSanitize { .. } => "alnum_sanitize",
        ClassifierModifier::DateFormat { .. } => "date_format",
        ClassifierModifier::Add { .. } => "add",
        ClassifierModifier::Sub { .. } => "sub",
        ClassifierModifier::Mul { .. } => "mul",
        ClassifierModifier::Div { .. } => "div",
    }
}

fn apply_modifier(
    snippets: &HashMap<u32, String>,
    modifier: &ClassifierModifier,
    computed_actions: &HashMap<String, String>,
) -> Result<Option<(u32, String)>, String> {
    match modifier {
        ClassifierModifier::Metadata { to, slug } => {
            let value = computed_actions
                .get(slug)
                .cloned()
                .ok_or_else(|| format!("computed action not found for slug '{slug}'"))?;
            Ok(Some((*to, value)))
        }
        ClassifierModifier::MonthNumber { from, to } => {
            let value = apply_replacements(from, snippets);
            let transformed =
                mod_month_number(&value).ok_or_else(|| format!("invalid month value '{value}'"))?;
            Ok(Some((*to, transformed)))
        }
        ClassifierModifier::MonthEnd { from, to } => {
            let value = apply_replacements(from, snippets);
            let transformed = mod_month_end(&value, None).map_err(|e| e.to_string())?;
            Ok(Some((*to, transformed)))
        }
        ClassifierModifier::MonthStart { from, to } => {
            let value = apply_replacements(from, snippets);
            let transformed = mod_month_start(&value, None).map_err(|e| e.to_string())?;
            Ok(Some((*to, transformed)))
        }
        ClassifierModifier::NextDay { from, to } => {
            let value = apply_replacements(from, snippets);
            let transformed = mod_next_day(&value, None).map_err(|e| e.to_string())?;
            Ok(Some((*to, transformed)))
        }
        ClassifierModifier::PrevDay { from, to } => {
            let value = apply_replacements(from, snippets);
            let transformed = mod_prev_day(&value, None).map_err(|e| e.to_string())?;
            Ok(Some((*to, transformed)))
        }
        ClassifierModifier::NextMonth { from, to } => {
            let value = apply_replacements(from, snippets);
            let transformed = mod_next_month(&value, None).map_err(|e| e.to_string())?;
            Ok(Some((*to, transformed)))
        }
        ClassifierModifier::PrevMonth { from, to } => {
            let value = apply_replacements(from, snippets);
            let transformed = mod_prev_month(&value, None).map_err(|e| e.to_string())?;
            Ok(Some((*to, transformed)))
        }
        ClassifierModifier::TaxYear { from, to } => {
            let value = apply_replacements(from, snippets);
            let transformed = mod_tax_year(&value).map_err(|e| e.to_string())?;
            Ok(Some((*to, transformed)))
        }
        ClassifierModifier::Currency { from, to } => {
            let value = apply_replacements(from, snippets);
            Ok(Some((*to, mod_currency(&value))))
        }
        ClassifierModifier::Sprintf { from, to, format } => {
            let value = apply_replacements(from, snippets);
            Ok(Some((*to, mod_sprintf(&value, format)?)))
        }
        ClassifierModifier::ZeroPad { from, to, length } => {
            let value = apply_replacements(from, snippets);
            Ok(Some((*to, mod_zeropad(&value, *length))))
        }
        ClassifierModifier::Replace { from, to } => {
            let value = apply_replacements(from, snippets);
            Ok(Some((*to, value)))
        }
        ClassifierModifier::AlnumSanitize { from, to } => {
            let value = apply_replacements(from, snippets);
            Ok(Some((*to, mod_alnum_sanitize(&value))))
        }
        ClassifierModifier::DateFormat { from, to, format } => {
            let value = apply_replacements(from, snippets);
            let transformed = mod_date_format(&value, Some(format)).map_err(|e| e.to_string())?;
            Ok(Some((*to, transformed)))
        }
        ClassifierModifier::Add { from, to } => {
            let transformed = mod_add(snippets, *from, *to)?;
            Ok(Some((*to, transformed)))
        }
        ClassifierModifier::Sub { from, to } => {
            let transformed = mod_sub(snippets, *from, *to)?;
            Ok(Some((*to, transformed)))
        }
        ClassifierModifier::Mul { from, to } => {
            let transformed = mod_mul(snippets, *from, *to)?;
            Ok(Some((*to, transformed)))
        }
        ClassifierModifier::Div { from, to } => {
            let transformed = mod_div(snippets, *from, *to)?;
            Ok(Some((*to, transformed)))
        }
    }
}

fn snippet_number(snippets: &HashMap<u32, String>, index: u32) -> Result<f64, String> {
    let value = snippets
        .get(&index)
        .ok_or_else(|| format!("missing snippet at index {index}"))?;
    let normalized = value.trim().replace([',', '$'], "");
    normalized
        .parse::<f64>()
        .map_err(|_| format!("snippet at index {index} is not numeric: {value}"))
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

fn mod_add(snippets: &HashMap<u32, String>, from: u32, to: u32) -> Result<String, String> {
    Ok(format_number(
        snippet_number(snippets, to)? + snippet_number(snippets, from)?,
    ))
}

fn mod_sub(snippets: &HashMap<u32, String>, from: u32, to: u32) -> Result<String, String> {
    Ok(format_number(
        snippet_number(snippets, to)? - snippet_number(snippets, from)?,
    ))
}

fn mod_mul(snippets: &HashMap<u32, String>, from: u32, to: u32) -> Result<String, String> {
    Ok(format_number(
        snippet_number(snippets, to)? * snippet_number(snippets, from)?,
    ))
}

fn mod_div(snippets: &HashMap<u32, String>, from: u32, to: u32) -> Result<String, String> {
    let denominator = snippet_number(snippets, from)?;
    if denominator.abs() < f64::EPSILON {
        return Err(format!("division by zero from snippet index {from}"));
    }

    Ok(format_number(snippet_number(snippets, to)? / denominator))
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
    tracing::debug!(
        value,
        result = &result,
        "classification: modifier: month_number"
    );
    result
}

fn mod_sprintf(value: &str, fmt: &str) -> Result<String, String> {
    tracing::debug!(value, fmt, "classification: modifier: sprintf");
    let mut v = value;
    let re = Regex::new(r"^0+([1-9])").unwrap();
    if let Some(caps) = re.captures(value) {
        if let Some(m) = caps.get(1) {
            v = m.as_str();
        }
    }

    sprintf!(fmt, v)
        .map(|value| value.replace(" ", "0"))
        .map_err(|err| err.to_string())
}

fn mod_zeropad(value: &str, length: usize) -> String {
    tracing::debug!(value, length, "classification: modifier: zero_pad");
    if value.len() >= length {
        return value.to_string();
    }

    let mut out = String::with_capacity(length);
    out.push_str(&"0".repeat(length - value.len()));
    out.push_str(value);
    out
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

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::HashMap;

    use chrono::Utc;

    use crate::domain::classifier_blocks::{ClassifierModifier, ClassifierRules};
    use crate::domain::documents::DocumentView;

    fn build_document_view(metadata: &[(&str, &str)]) -> DocumentView {
        DocumentView {
            id: 1,
            title: "Test Document".to_string(),
            document_type_id: 1,
            pages: 1,
            metadata: metadata
                .iter()
                .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
                .collect(),
            cabinet_ids: Vec::new(),
            tag_ids: Vec::new(),
            created_by: 1,
            created_at: Utc::now(),
            updated_by: 1,
            updated_at: Utc::now(),
        }
    }

    fn build_rules(
        continue_after_match: bool,
        match_patterns: Vec<ClassifierPattern>,
        match_actions: &[(&str, &str)],
    ) -> ClassifierRules {
        ClassifierRules {
            continue_after_match,
            match_patterns,
            match_actions: match_actions
                .iter()
                .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
                .collect(),
            child_rules: Vec::new(),
        }
    }

    fn build_block(id: i64, order: i32, rules: ClassifierRules) -> ClassifierBlock {
        ClassifierBlock {
            id,
            name: format!("Block {id}"),
            description: None,
            enabled: true,
            order,
            rules: diesel_json::Json(rules),
            created_by: 1,
            created_at: Utc::now(),
            updated_by: 1,
            updated_at: Utc::now(),
        }
    }

    fn apply_modifier_for_test(
        modifier: ClassifierModifier,
        snippets: &[(u32, &str)],
        computed_actions: &[(&str, &str)],
    ) -> HashMap<u32, String> {
        let mut snippets: HashMap<u32, String> = snippets
            .iter()
            .map(|(index, value)| (*index, (*value).to_string()))
            .collect();
        let computed_actions: HashMap<String, String> = computed_actions
            .iter()
            .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
            .collect();

        if let Ok(Some((to, value))) = apply_modifier(&snippets, &modifier, &computed_actions) {
            snippets.insert(to, value);
        }
        snippets
    }

    #[test]
    fn metadata_matching_prefers_computed_actions_over_document_metadata() {
        let document = build_document_view(&[("status", "fallback")]);
        let computed_actions = HashMap::from([("status".to_string(), "computed".to_string())]);
        let pattern = ClassifierPattern {
            text: None,
            metadata: Some(HashMap::from([(
                "status".to_string(),
                "computed".to_string(),
            )])),
        };

        let matched = does_document_match_pattern(&document, "", &computed_actions, &pattern)
            .expect("pattern match should succeed");

        assert!(matches!(matched, PatternMatch::Metadata));
    }

    #[test]
    fn metadata_matching_does_not_fallback_when_computed_action_key_exists() {
        let document = build_document_view(&[("status", "fallback")]);
        let computed_actions = HashMap::from([("status".to_string(), "computed".to_string())]);
        let pattern = ClassifierPattern {
            text: None,
            metadata: Some(HashMap::from([(
                "status".to_string(),
                "fallback".to_string(),
            )])),
        };

        let matched = does_document_match_pattern(&document, "", &computed_actions, &pattern)
            .expect("pattern match should succeed");

        assert!(matches!(matched, PatternMatch::None));
    }

    #[test]
    fn text_matching_is_case_insensitive() {
        let document = build_document_view(&[]);
        let computed_actions = HashMap::new();
        let pattern = ClassifierPattern {
            text: Some("invoice #([0-9]+)".to_string()),
            metadata: None,
        };

        let matched =
            does_document_match_pattern(&document, "INVOICE #123", &computed_actions, &pattern)
                .expect("pattern match should succeed");

        match matched {
            PatternMatch::Text(captures) => assert_eq!(&captures[1], "123"),
            _ => panic!("expected text match"),
        }
    }

    #[test]
    fn compute_classification_actions_continues_and_overwrites_later_blocks() {
        let document = build_document_view(&[]);
        let blocks = vec![
            build_block(
                1,
                1,
                build_rules(
                    true,
                    vec![ClassifierPattern {
                        text: Some("Invoice".to_string()),
                        metadata: None,
                    }],
                    &[("stage", "first"), ("shared", "from-first")],
                ),
            ),
            build_block(
                2,
                2,
                build_rules(
                    false,
                    vec![ClassifierPattern {
                        text: None,
                        metadata: Some(HashMap::from([("stage".to_string(), "first".to_string())])),
                    }],
                    &[("shared", "from-second"), ("final", "done")],
                ),
            ),
        ];

        let computed_actions =
            compute_classification_actions(document.id, &document, "Invoice #123", &blocks)
                .expect("classification should succeed");

        assert_eq!(computed_actions.get("stage"), Some(&"first".to_string()));
        assert_eq!(
            computed_actions.get("shared"),
            Some(&"from-second".to_string())
        );
        assert_eq!(computed_actions.get("final"), Some(&"done".to_string()));
    }

    #[test]
    fn compute_classification_actions_stops_when_continue_after_match_is_false() {
        let document = build_document_view(&[]);
        let blocks = vec![
            build_block(
                1,
                1,
                build_rules(
                    false,
                    vec![ClassifierPattern {
                        text: Some("Invoice".to_string()),
                        metadata: None,
                    }],
                    &[("stage", "first")],
                ),
            ),
            build_block(
                2,
                2,
                build_rules(
                    false,
                    vec![ClassifierPattern {
                        text: Some("Invoice".to_string()),
                        metadata: None,
                    }],
                    &[("stage", "second")],
                ),
            ),
        ];

        let computed_actions =
            compute_classification_actions(document.id, &document, "Invoice #123", &blocks)
                .expect("classification should succeed");

        assert_eq!(computed_actions.get("stage"), Some(&"first".to_string()));
    }

    #[test]
    fn modifier_metadata_copies_from_computed_actions() {
        let snippets = apply_modifier_for_test(
            ClassifierModifier::Metadata {
                to: 2,
                slug: "invoice_number".to_string(),
            },
            &[],
            &[("invoice_number", "123")],
        );

        assert_eq!(snippets.get(&2), Some(&"123".to_string()));
    }

    #[test]
    fn modifier_month_number_formats_full_and_abbreviated_months() {
        let full = apply_modifier_for_test(
            ClassifierModifier::MonthNumber {
                from: "\\1".to_string(),
                to: 2,
            },
            &[(1, "January")],
            &[],
        );
        let abbreviated = apply_modifier_for_test(
            ClassifierModifier::MonthNumber {
                from: "\\1".to_string(),
                to: 2,
            },
            &[(1, "sep")],
            &[],
        );

        assert_eq!(full.get(&2), Some(&"01".to_string()));
        assert_eq!(abbreviated.get(&2), Some(&"09".to_string()));
    }

    #[test]
    fn modifier_month_end_returns_last_day_of_month() {
        let snippets = apply_modifier_for_test(
            ClassifierModifier::MonthEnd {
                from: "\\1".to_string(),
                to: 2,
            },
            &[(1, "2024-02-10")],
            &[],
        );

        assert_eq!(snippets.get(&2), Some(&"2024-02-29".to_string()));
    }

    #[test]
    fn modifier_month_start_returns_first_day_of_month() {
        let snippets = apply_modifier_for_test(
            ClassifierModifier::MonthStart {
                from: "\\1".to_string(),
                to: 2,
            },
            &[(1, "2024-02-10")],
            &[],
        );

        assert_eq!(snippets.get(&2), Some(&"2024-02-01".to_string()));
    }

    #[test]
    fn modifier_next_day_supports_default_and_explicit_offsets() {
        let default_offset = apply_modifier_for_test(
            ClassifierModifier::NextDay {
                from: "\\1".to_string(),
                to: 2,
            },
            &[(1, "2024-01-10")],
            &[],
        );
        let explicit_offset = apply_modifier_for_test(
            ClassifierModifier::NextDay {
                from: "2|\\1".to_string(),
                to: 2,
            },
            &[(1, "2024-01-10")],
            &[],
        );

        assert_eq!(default_offset.get(&2), Some(&"2024-01-11".to_string()));
        assert_eq!(explicit_offset.get(&2), Some(&"2024-01-12".to_string()));
    }

    #[test]
    fn modifier_prev_day_supports_default_and_explicit_offsets() {
        let default_offset = apply_modifier_for_test(
            ClassifierModifier::PrevDay {
                from: "\\1".to_string(),
                to: 2,
            },
            &[(1, "2024-01-10")],
            &[],
        );
        let explicit_offset = apply_modifier_for_test(
            ClassifierModifier::PrevDay {
                from: "2|\\1".to_string(),
                to: 2,
            },
            &[(1, "2024-01-10")],
            &[],
        );

        assert_eq!(default_offset.get(&2), Some(&"2024-01-09".to_string()));
        assert_eq!(explicit_offset.get(&2), Some(&"2024-01-08".to_string()));
    }

    #[test]
    fn modifier_next_month_clamps_to_valid_day() {
        let snippets = apply_modifier_for_test(
            ClassifierModifier::NextMonth {
                from: "\\1".to_string(),
                to: 2,
            },
            &[(1, "2024-01-31")],
            &[],
        );

        assert_eq!(snippets.get(&2), Some(&"2024-02-29".to_string()));
    }

    #[test]
    fn modifier_prev_month_clamps_to_valid_day() {
        let snippets = apply_modifier_for_test(
            ClassifierModifier::PrevMonth {
                from: "\\1".to_string(),
                to: 2,
            },
            &[(1, "2024-03-31")],
            &[],
        );

        assert_eq!(snippets.get(&2), Some(&"2024-02-29".to_string()));
    }

    #[test]
    fn modifier_tax_year_uses_shifted_month_year() {
        let snippets = apply_modifier_for_test(
            ClassifierModifier::TaxYear {
                from: "\\1".to_string(),
                to: 2,
            },
            &[(1, "2024-12-31")],
            &[],
        );

        assert_eq!(snippets.get(&2), Some(&"2025".to_string()));
    }

    #[test]
    fn modifier_currency_normalizes_currency_text() {
        let snippets = apply_modifier_for_test(
            ClassifierModifier::Currency {
                from: "\\1".to_string(),
                to: 2,
            },
            &[(1, "$001,234")],
            &[],
        );

        assert_eq!(snippets.get(&2), Some(&"1234".to_string()));
    }

    #[test]
    fn modifier_sprintf_formats_and_zero_pads() {
        let snippets = apply_modifier_for_test(
            ClassifierModifier::Sprintf {
                from: "\\1".to_string(),
                to: 2,
                format: "%4s".to_string(),
            },
            &[(1, "007")],
            &[],
        );

        assert_eq!(snippets.get(&2), Some(&"0007".to_string()));
    }

    #[test]
    fn modifier_sprintf_returns_error_for_invalid_format() {
        let snippets = HashMap::from([(1, "7".to_string())]);
        let computed_actions = HashMap::new();
        let modifier = ClassifierModifier::Sprintf {
            from: "\\1".to_string(),
            to: 2,
            format: "%q".to_string(),
        };

        let result = apply_modifier(&snippets, &modifier, &computed_actions);

        assert!(result.is_err());
    }

    #[test]
    fn modifier_zero_pad_left_pads_to_requested_length() {
        let snippets = apply_modifier_for_test(
            ClassifierModifier::ZeroPad {
                from: "\\1".to_string(),
                to: 2,
                length: 4,
            },
            &[(1, "7")],
            &[],
        );

        assert_eq!(snippets.get(&2), Some(&"0007".to_string()));
    }

    #[test]
    fn modifier_zero_pad_keeps_equal_or_longer_values() {
        let equal = apply_modifier_for_test(
            ClassifierModifier::ZeroPad {
                from: "\\1".to_string(),
                to: 2,
                length: 4,
            },
            &[(1, "1234")],
            &[],
        );
        let longer = apply_modifier_for_test(
            ClassifierModifier::ZeroPad {
                from: "\\1".to_string(),
                to: 2,
                length: 4,
            },
            &[(1, "12345")],
            &[],
        );

        assert_eq!(equal.get(&2), Some(&"1234".to_string()));
        assert_eq!(longer.get(&2), Some(&"12345".to_string()));
    }

    #[test]
    fn modifier_zero_pad_supports_empty_values() {
        let snippets = apply_modifier_for_test(
            ClassifierModifier::ZeroPad {
                from: "\\1".to_string(),
                to: 2,
                length: 3,
            },
            &[(1, "")],
            &[],
        );

        assert_eq!(snippets.get(&2), Some(&"000".to_string()));
    }

    #[test]
    fn modifier_replace_applies_snippet_replacements() {
        let snippets = apply_modifier_for_test(
            ClassifierModifier::Replace {
                from: "INV-\\1".to_string(),
                to: 2,
            },
            &[(1, "123")],
            &[],
        );

        assert_eq!(snippets.get(&2), Some(&"INV-123".to_string()));
    }

    #[test]
    fn modifier_alnum_sanitize_removes_punctuation_and_compacts_whitespace() {
        let snippets = apply_modifier_for_test(
            ClassifierModifier::AlnumSanitize {
                from: "\\1".to_string(),
                to: 2,
            },
            &[(1, " ACME-123 / West ")],
            &[],
        );

        assert_eq!(snippets.get(&2), Some(&"ACME123 West".to_string()));
    }

    #[test]
    fn modifier_date_format_uses_requested_output_format() {
        let snippets = apply_modifier_for_test(
            ClassifierModifier::DateFormat {
                from: "\\1".to_string(),
                to: 2,
                format: "%m/%d/%Y".to_string(),
            },
            &[(1, "2024-01-10")],
            &[],
        );

        assert_eq!(snippets.get(&2), Some(&"01/10/2024".to_string()));
    }

    #[test]
    fn modifier_add_sums_numeric_snippets() {
        let snippets = apply_modifier_for_test(
            ClassifierModifier::Add { from: 1, to: 2 },
            &[(1, "2"), (2, "10")],
            &[],
        );

        assert_eq!(snippets.get(&2), Some(&"12".to_string()));
    }

    #[test]
    fn modifier_sub_subtracts_numeric_snippets() {
        let snippets = apply_modifier_for_test(
            ClassifierModifier::Sub { from: 1, to: 2 },
            &[(1, "2"), (2, "10")],
            &[],
        );

        assert_eq!(snippets.get(&2), Some(&"8".to_string()));
    }

    #[test]
    fn modifier_mul_multiplies_numeric_snippets() {
        let snippets = apply_modifier_for_test(
            ClassifierModifier::Mul { from: 1, to: 2 },
            &[(1, "2"), (2, "10")],
            &[],
        );

        assert_eq!(snippets.get(&2), Some(&"20".to_string()));
    }

    #[test]
    fn modifier_div_divides_numeric_snippets() {
        let snippets = apply_modifier_for_test(
            ClassifierModifier::Div { from: 1, to: 2 },
            &[(1, "2"), (2, "10")],
            &[],
        );

        assert_eq!(snippets.get(&2), Some(&"5".to_string()));
    }

    #[test]
    fn modifier_invalid_inputs_do_not_write_output() {
        let missing_metadata = apply_modifier_for_test(
            ClassifierModifier::Metadata {
                to: 2,
                slug: "missing".to_string(),
            },
            &[],
            &[],
        );
        let invalid_month = apply_modifier_for_test(
            ClassifierModifier::MonthNumber {
                from: "\\1".to_string(),
                to: 2,
            },
            &[(1, "NotAMonth")],
            &[],
        );
        let invalid_date = apply_modifier_for_test(
            ClassifierModifier::DateFormat {
                from: "\\1".to_string(),
                to: 2,
                format: "%m/%d/%Y".to_string(),
            },
            &[(1, "not-a-date")],
            &[],
        );
        let divide_by_zero = apply_modifier_for_test(
            ClassifierModifier::Div { from: 1, to: 2 },
            &[(1, "0"), (2, "10")],
            &[],
        );

        assert!(!missing_metadata.contains_key(&2));
        assert!(!invalid_month.contains_key(&2));
        assert!(!invalid_date.contains_key(&2));
        assert_eq!(divide_by_zero.get(&2), Some(&"10".to_string()));
    }
}
