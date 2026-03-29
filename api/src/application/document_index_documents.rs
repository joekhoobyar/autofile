use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use bb8::PooledConnection;
use serde::{Deserialize, Serialize};

use apalis::prelude::*;
use diesel::prelude::*;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::{AsyncPgConnection, RunQueryDsl};

use crate::application::documents::get_document_view;
use crate::domain::document_indexes::{DocumentIndex, DocumentIndexTemplate};
use crate::domain::documents::DocumentView;
use crate::schema::{
    document_index_documents,
    document_index_templates,
    document_index_values,
    document_indexes,
};
use crate::shared::app_state::AppState;
use crate::shared::util::{to_job_error, ApiError};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateDocumentIndexDocument {
    pub document_index_id: i64,
    pub document_id: i64,
}

/**
 * This function is responsible for enqueueing jobs to update all document indexes for a given document.
 */
pub async fn enqueue_document_index_document_updates(
    document_id: i64,
    state: Arc<AppState>,
) -> Result<(), ApiError> {
    tracing::info!(?document_id, "Enqueueing document_index updates for document {:?}", document_id);
    let mut db = state
        .db_pool
        .get()
        .await
        .map_err(|e| ApiError::internal_server_error(&format!("Failed to fetch db connection: {}", e)))?;

    // Load the document index ids for all document_index rows where enabled = true.
    let index_ids = document_indexes::table
        .filter(document_indexes::enabled.eq(true))
        .select(document_indexes::id)
        .load::<i64>(&mut db)
        .await
        .map_err(|e| ApiError::internal_server_error(&format!("Failed to fetch document indexes: {}", e)))?;

    // Enqueue a job for each document index, passing the document_id and document_index_id.
    let mut index_jobs = state.index_jobs.as_ref().clone();
    for index_id in index_ids {
        index_jobs
            .push(UpdateDocumentIndexDocument {
                document_index_id: index_id,
                document_id,
            })
            .await
            .map_err(|e| ApiError::internal_server_error(&format!("Failed to enqueue index job: {}", e)))?;
    }

    Ok(())
}

/**
 * This job updates a document index for a given document.
 * 
 * It is responsible for adding, updating or removing the document from relevant index nodes.
 */
pub async fn update_document_index_document(
    job: UpdateDocumentIndexDocument,
    state: Data<Arc<AppState>>,
) -> Result<(), Error> {
    tracing::info!(?job, "Updating document_index {:?} for document {:?}", job.document_index_id, job.document_id);

    let document_view = {
        let db = state
            .db_pool
            .get()
            .await
            .map_err(to_job_error)?;
        get_document_view(db, job.document_id)
            .await
            .map_err(to_job_error)?
    };

    let mut db = state
        .db_pool
        .get()
        .await
        .map_err(to_job_error)?;

    // Load the document index.
    let document_index = document_indexes::table
        .find(job.document_index_id)
        .select(DocumentIndex::as_select())
        .first::<DocumentIndex>(&mut db)
        .await;
    if matches!(document_index, Err(diesel::result::Error::NotFound)) {
        tracing::info!(
            document_index_id = job.document_index_id,
            "document_index {:?} missing; skipping update", job.document_index_id
        );
        return Ok(());
    }
    
    // Find any existing document_index_document records for this document.
    // We want the document_index_value_id(s), but only for this document_index_id.
    // That will require joining document_index_documents to document_index_values to document_index_templates to filter by document_index_id.
    // We will collect these values in a Set.
    let existing_value_ids = document_index_documents::table
        .inner_join(
            document_index_values::table.on(
                document_index_documents::document_index_value_id.eq(document_index_values::id),
            ),
        )
        .inner_join(
            document_index_templates::table.on(
                document_index_values::document_index_template_id.eq(document_index_templates::id),
            ),
        )
        .filter(document_index_documents::document_id.eq(job.document_id))
        .filter(document_index_templates::document_index_id.eq(job.document_index_id))
        .select(document_index_values::id)
        .load::<i64>(&mut db)
        .await
        .map_err(to_job_error)?;
    let mut existing_value_ids: HashSet<i64> = existing_value_ids.into_iter().collect();

    // Next, start at the root(s) of the document index (the template with no parent) and traverse down the tree, matching the document's metadata to the template's criteria, and updating the document_index_documents records as needed.
    // Call apply_document_index_value for each root template, passing the set of existing document_index_value_ids.
    // This function will evaluate the template against the DocumentView, using minijinja.
    let templates = document_index_templates::table
        .filter(document_index_templates::document_index_id.eq(job.document_index_id))
        .select(DocumentIndexTemplate::as_select())
        .load::<DocumentIndexTemplate>(&mut db)
        .await
        .map_err(to_job_error)?;

    // Build a parent -> children index for templates so we can traverse the tree without recursion.
    // The key is parent_id (None for roots), and the value is a list of indices into `templates`.
    let mut children_by_parent: HashMap<Option<i64>, Vec<usize>> = HashMap::new();
    for (idx, template) in templates.iter().enumerate() {
        children_by_parent
            .entry(template.parent_id)
            .or_default()
            .push(idx);
    }

    // Seed the stack with root templates (parent_id = None). Each stack entry carries the
    // template index plus the parent document_index_value_id to attach to (None at root).
    let mut stack: Vec<(usize, Option<i64>)> = Vec::new();
    if let Some(root_ids) = children_by_parent.get(&None) {
        for &idx in root_ids {
            stack.push((idx, None));
        }
    }

    // Depth-first traversal using an explicit stack to avoid recursive async calls.
    while let Some((idx, parent_value_id)) = stack.pop() {
        let template = &templates[idx];
        let value_id = apply_document_index_value(
            &mut db,
            &document_view,
            template,
            &mut existing_value_ids,
            parent_value_id,
        )
        .await?;

        if !template.is_leaf {
            // Push children onto the stack with this node's value_id as their parent.
            if let Some(child_ids) = children_by_parent.get(&Some(template.id)) {
                for &child_idx in child_ids {
                    stack.push((child_idx, Some(value_id)));
                }
            }
        }
    }

    // At the end of processing all templates, any document_index_value_ids remaining in the set should be deleted,
    // as they no longer match the document.
    if !existing_value_ids.is_empty() {
        diesel::delete(
            document_index_documents::table.filter(
                document_index_documents::document_id
                    .eq(job.document_id)
                    .and(document_index_documents::document_index_value_id.eq_any(existing_value_ids)),
            ),
        )
        .execute(&mut db)
        .await
        .map_err(to_job_error)?;
    }

    Ok(())
}

pub async fn update_document_index_document_logged(
    job: UpdateDocumentIndexDocument,
    state: Data<Arc<AppState>>,
) -> Result<(), Error> {
    match update_document_index_document(job.clone(), state).await {
        Ok(()) => Ok(()),
        Err(err) => {
            tracing::error!(error = %err, "document_index {:?} update job failed for document {:?}", job.document_index_id, job.document_id);
            Err(err)
        }
    }
}

async fn apply_document_index_value(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    doc: &DocumentView,
    template: &DocumentIndexTemplate,
    original_value_ids: &mut HashSet<i64>,
    parent_value_id: Option<i64>,
) -> Result<i64, Error> {
    // Evaluate the template against the DocumentView, using minijinja
    // We will pass this DocumentView to minijinja under the "doc" key.
    let env = minijinja::Environment::new();
    let rendered_value = env
        .render_str(&template.template, minijinja::context! { doc => doc })
        .map_err(to_job_error)?;

    // Upsert the document_index_values record for the evaluated text value.
    let value_id: i64 = diesel::insert_into(document_index_values::table)
        .values((
            document_index_values::value.eq(rendered_value),
            document_index_values::document_index_template_id.eq(template.id),
            document_index_values::parent_id.eq(parent_value_id),
        ))
        .on_conflict((
            document_index_values::document_index_template_id,
            document_index_values::value,
        ))
        .do_update()
        .set(document_index_values::value.eq(diesel::upsert::excluded(document_index_values::value)))
        .returning(document_index_values::id)
        .get_result(db)
        .await
        .map_err(to_job_error)?;

    // If this is a leaf node, we need upsert a document_index_documents record for this document and the document_index_value_id we just upserted
    // We then remove the document_index_value_id from the set existing document_index_value_ids, if it exists.
    // At the end of processing all templates, any document_index_value_ids remaining in the set should be deleted,
    // as they no longer match the document.
    if template.is_leaf {
        diesel::insert_into(document_index_documents::table)
            .values((
                document_index_documents::document_index_value_id.eq(value_id),
                document_index_documents::document_id.eq(doc.id),
            ))
            .on_conflict((
                document_index_documents::document_index_value_id,
                document_index_documents::document_id,
            ))
            .do_nothing()
            .execute(db)
            .await
            .map_err(to_job_error)?;

        original_value_ids.remove(&value_id);
    }

    Ok(value_id)
}
