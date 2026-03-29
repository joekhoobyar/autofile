use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use apalis::prelude::*;
use bb8::PooledConnection;
use diesel::prelude::*;
use diesel::dsl::{exists, not};
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::{AsyncPgConnection, RunQueryDsl};

use crate::application::documents::get_document_view;
use crate::domain::document_indexes::{DocumentIndex, DocumentIndexTemplate};
use crate::domain::documents::{DocumentView, TemplateDocumentView};
use crate::schema::{
    cabinets,
    document_index_documents,
    document_index_templates,
    document_index_values,
    document_indexes,
    document_types,
    tags,
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
    tracing::info!(?document_id, "Enqueueing document_index updates");
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
    match do_update_document_index_document(job.clone(), state).await {
        Ok(()) => Ok(()),
        Err(err) => {
            tracing::error!(error = %err, "document_index {:?} update job failed for document {:?}", job.document_index_id, job.document_id);
            Err(err)
        }
    }
}

/**
 * Internal function to update a document index for a given document.
 * 
 * Separated from the public function to allow for more granular error handling and logging.
 */
async fn do_update_document_index_document(
    job: UpdateDocumentIndexDocument,
    state: Data<Arc<AppState>>,
) -> Result<(), Error> {
    tracing::info!(?job, "Updating document_index for document");

    let mut db = state
        .db_pool
        .get()
        .await
        .map_err(to_job_error)?;

    let document_view = get_document_view(&mut db, job.document_id)
        .await
        .map_err(to_job_error)?;

    let template_document_view = build_template_document_view(&mut db, document_view).await?;

    // Check if the document index still exists - if not, we can skip this operation entirely.
    let document_index = document_indexes::table
        .find(job.document_index_id)
        .select(DocumentIndex::as_select())
        .first::<DocumentIndex>(&mut db)
        .await;
    if matches!(document_index, Err(diesel::result::Error::NotFound)) {
        tracing::info!(
            document_index_id = job.document_index_id,
            "document_index missing; skipping update"
        );
        return Ok(());
    }
    
    let document_id = job.document_id;
    let document_index_id = job.document_index_id;
    let document_view = template_document_view;

    let skip_due_to_empty_template = Arc::new(AtomicBool::new(false));
    let skip_due_to_empty_template_tx = Arc::clone(&skip_due_to_empty_template);
    let skip_due_to_no_leaf = Arc::new(AtomicBool::new(false));
    let skip_due_to_no_leaf_tx = Arc::clone(&skip_due_to_no_leaf);

    let tx_result = db.build_transaction().run::<_, diesel::result::Error, _>(|conn| {
        Box::pin(async move {
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
                .filter(document_index_documents::document_id.eq(document_id))
                .filter(document_index_templates::document_index_id.eq(document_index_id))
                .select(document_index_values::id)
                .load::<i64>(conn)
                .await?;
            let mut existing_value_ids: HashSet<i64> = existing_value_ids.into_iter().collect();

            // Next, start at the root(s) of the document index (the template with no parent) and traverse down the tree, matching the document's metadata to the template's criteria, and updating the document_index_documents records as needed.
            // Call apply_document_index_value for each root template, passing the set of existing document_index_value_ids.
            // This function will evaluate the template against the DocumentView, using minijinja.
            let templates = document_index_templates::table
                .filter(document_index_templates::document_index_id.eq(document_index_id))
                .select(DocumentIndexTemplate::as_select())
                .load::<DocumentIndexTemplate>(conn)
                .await?;

            // Build a parent -> children index for templates so we can traverse the tree without recursion.
            // The key is parent_id (None for roots), and the value is a list of indices into `templates`.
            let mut children_by_parent: HashMap<Option<i64>, Vec<usize>> = HashMap::new();
            for (idx, template) in templates.iter().enumerate() {
                children_by_parent
                    .entry(template.parent_id)
                    .or_default()
                    .push(idx);
            }

            // Seed the stack with root templates (parent_id = None). Each root is processed independently
            // so we can detect whether any leaf template exists for that root.
            if let Some(root_ids) = children_by_parent.get(&None) {
                for &root_idx in root_ids {
                    let mut stack: Vec<(usize, Option<i64>)> = vec![(root_idx, None)];
                    let mut leaf_found = false;

                    // Depth-first traversal using an explicit stack to avoid recursive async calls.
                    while let Some((idx, parent_value_id)) = stack.pop() {
                        let template = &templates[idx];

                        let value_id = apply_document_index_value(
                            conn,
                            &document_view,
                            template,
                            &mut existing_value_ids,
                            parent_value_id,
                        )
                        .await
                        .map_err(|err| {
                            tracing::error!(error = %err, "document_index_values apply failed inside transaction");
                            err
                        })?;

                        let Some(value_id) = value_id else {
                            skip_due_to_empty_template_tx.store(true, Ordering::Relaxed);
                            return Err(diesel::result::Error::RollbackTransaction);
                        };

                        if !template.is_leaf {
                            // Push children onto the stack with this node's value_id as their parent.
                            if let Some(child_ids) = children_by_parent.get(&Some(template.id)) {
                                for &child_idx in child_ids {
                                    stack.push((child_idx, Some(value_id)));
                                }
                            }
                        } else {
                            leaf_found = true;
                        }
                    }

                    if !leaf_found {
                        skip_due_to_no_leaf_tx.store(true, Ordering::Relaxed);
                        return Err(diesel::result::Error::RollbackTransaction);
                    }
                }
            }

            // At the end of processing all templates, any document_index_value_ids remaining in the set should be deleted,
            // as they no longer match the document.
            delete_stale_document_index_values(conn, document_id, &existing_value_ids)
                .await
                .map_err(|err| {
                    tracing::error!(error = %err, "document_index_values cleanup failed inside transaction");
                    err
                })?;

            Ok(())
        })
    })
        .await;

    // Skip logging errors for transactions that were rolled back due to empty templates or no leaf templates,
    // as these are expected to occur and not indicative of a system issue.
    if let Err(diesel::result::Error::RollbackTransaction) = tx_result {
        let should_skip =
            skip_due_to_empty_template.as_ref().load(Ordering::Relaxed)
                || skip_due_to_no_leaf.as_ref().load(Ordering::Relaxed);
        if should_skip {
            return Ok(());
        }
    }

    tx_result.map_err(to_job_error)?;

    Ok(())
}

/**
 * Internal function to build a TemplateDocumentView for a given DocumentView,
 * by loading the document type slug, tag slugs and cabinet slugs.
 * 
 * This is needed to evaluate the document index templates, which may reference these fields.
 */
async fn build_template_document_view(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    document_view: DocumentView,
) -> Result<TemplateDocumentView, Error> {
    let document_type_slug = document_types::table
        .find(document_view.document_type_id)
        .select(document_types::slug)
        .first::<String>(db)
        .await
        .map_err(to_job_error)?;

    let tags: HashSet<String> = if document_view.tag_ids.is_empty() {
        HashSet::new()
    } else {
        tags::table
            .filter(tags::id.eq_any(&document_view.tag_ids))
            .select(tags::slug)
            .load::<String>(db)
            .await
            .map_err(to_job_error)?
            .into_iter()
            .collect()
    };

    let cabinets: HashSet<String> = if document_view.cabinet_ids.is_empty() {
        HashSet::new()
    } else {
        cabinets::table
            .filter(cabinets::id.eq_any(&document_view.cabinet_ids))
            .select(cabinets::slug)
            .load::<String>(db)
            .await
            .map_err(to_job_error)?
            .into_iter()
            .collect()
    };

    Ok(TemplateDocumentView {
        id: document_view.id,
        title: document_view.title,
        document_type_id: document_view.document_type_id,
        document_type: document_type_slug,
        metadata: document_view.metadata,
        cabinet_ids: document_view.cabinet_ids,
        tag_ids: document_view.tag_ids,
        cabinets,
        tags,
        created_by: document_view.created_by,
        created_at: document_view.created_at,
        updated_by: document_view.updated_by,
        updated_at: document_view.updated_at,
    })
}

/**
 * Internal function to delete document_index_document records for a given document_id
 * and a set of document_index_value_ids, as well as any document_index_value records
 * that are no longer linked to any document after the deletion, and their ancestor values
 * if they no longer have children.
 */
async fn delete_stale_document_index_values(
    db: &mut AsyncPgConnection,
    document_id: i64,
    remaining_value_ids: &HashSet<i64>,
) -> Result<(), diesel::result::Error> {
    if remaining_value_ids.is_empty() {
        return Ok(());
    }

    let remaining_value_ids: Vec<i64> = remaining_value_ids.iter().copied().collect();

    // Delete the original document links.
    diesel::delete(
        document_index_documents::table.filter(
            document_index_documents::document_id
                .eq(document_id)
                .and(document_index_documents::document_index_value_id.eq_any(&remaining_value_ids)),
        ),
    )
        .execute(db)
        .await?;
    // Delete the original value leaf nodes only if no more document_index_documents exist,
    // and collect their parent_id values for ancestor cleanup.
    let remaining_parent_ids: Vec<i64> = diesel::delete(
        document_index_values::table
            .filter(document_index_values::id.eq_any(&remaining_value_ids))
            .filter(not(exists(
                document_index_documents::table.filter(
                    document_index_documents::document_index_value_id.eq(document_index_values::id),
                ),
            ))),
    )
        .returning(document_index_values::parent_id)
        .get_results::<Option<i64>>(db)
        .await?
        .into_iter()
        .flatten()
        .collect();
    if remaining_parent_ids.is_empty() {
        return Ok(());
    }

    // Delete any ancestor values that no longer have children after leaf removal.
    diesel::sql_query(
        r#"
        WITH RECURSIVE deletable AS (
            SELECT t.id, t.parent_id
            FROM document_index_values t
            WHERE t.id = ANY($1)

            UNION ALL

            SELECT p.id, p.parent_id
            FROM document_index_values p
            JOIN deletable d
            ON p.id = d.parent_id
            WHERE NOT EXISTS (
                SELECT 1
                FROM document_index_values c
                WHERE c.parent_id = p.id
                AND c.id <> d.id
            )
        )
        DELETE FROM document_index_values
        WHERE id IN (SELECT id FROM deletable)
        "#
    )
        .bind::<diesel::sql_types::Array<diesel::sql_types::BigInt>, _>(remaining_parent_ids)
        .execute(db)
        .await?;

    Ok(())
}

/**
 * Internal function to evaluate a document index template for a document, which may result in
 * upserting document_index_value and document_index_document records.
 * 
 * Will remove the value_id from the set of existing_value_ids if a document_index_document record is upserted.
 */
async fn apply_document_index_value(
    db: &mut AsyncPgConnection,
    doc: &TemplateDocumentView,
    template: &DocumentIndexTemplate,
    original_value_ids: &mut HashSet<i64>,
    parent_value_id: Option<i64>,
) -> Result<Option<i64>, diesel::result::Error> {

    // Evaluate the template against the DocumentView, using minijinja
    // We will pass this DocumentView to minijinja under the "doc" key.
    let env = minijinja::Environment::new();
    let rendered_value = env
        .render_str(&template.template, minijinja::context! { doc })
        .map_err(|err| {
            tracing::error!(error = %err, "document_index_values template evaluation failed inside transaction");
            diesel::result::Error::RollbackTransaction
        })?;
    tracing::debug!({value = &rendered_value, template = template.template, tags = ?doc.tags}, "document_index_values template evaluation result");
    if rendered_value.trim().is_empty() {
        return Ok(None);
    }

    // Upsert the document_index_values record for the evaluated text value.
    let value_id: i64 = diesel::insert_into(document_index_values::table)
        .values((
            document_index_values::document_index_template_id.eq(template.id),
            document_index_values::value.eq(rendered_value),
            document_index_values::parent_id.eq(parent_value_id),
        ))
        .on_conflict((
            document_index_values::document_index_template_id,
            document_index_values::value,
        ))
        .do_nothing()
        .returning(document_index_values::id)
        .get_result(db)
        .await?;

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
            .await?;

        original_value_ids.remove(&value_id);
    }

    Ok(Some(value_id))
}
