use std::collections::HashMap;
use std::sync::Arc;

use apalis::prelude::*;
use bb8::PooledConnection;
use diesel::prelude::*;
use diesel_async::AsyncPgConnection;
use diesel_async::RunQueryDsl;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use regex::Captures;

use crate::application::documents::get_document_view;
use crate::domain::classifier_blocks::{ClassifierBlock, ClassifierPattern};
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
        let reg = regex::Regex::new(pattern_text)?;
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
