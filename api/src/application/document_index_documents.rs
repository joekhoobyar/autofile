use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;


use apalis::prelude::*;
use bb8::PooledConnection;
use diesel::prelude::*;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::{AsyncPgConnection, RunQueryDsl};

use crate::application::jobs::FastJob;
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
use crate::shared::util::{ApiError, JobResult};

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
    let mut fast_jobs = state.fast_jobs.as_ref().clone();
    for index_id in index_ids {
        fast_jobs
            .push(FastJob::UpdateDocumentIndexDocument {
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
    document_index_id: i64,
    document_id: i64,
    state: Data<Arc<AppState>>,
) -> Result<(), Error> {
    match do_update_document_index_document(document_index_id, document_id, state).await {
        Ok(()) => Ok(()),
        Err(err) => {
            tracing::error!(error = %err, "document_index {:?} update job failed for document {:?}", document_index_id, document_id);
            Err(err.into())
        }
    }
}

/**
 * Internal function to update a document index for a given document.
 * 
 * Separated from the public function to allow for more granular error handling and logging.
 */
async fn do_update_document_index_document(
    document_index_id: i64,
    document_id: i64,
    state: Data<Arc<AppState>>,
) -> JobResult<()> {
    tracing::info!(document_index_id, document_id, "Updating document_index for document");

    let mut db = state
        .db_pool
        .get()
        .await?;

    // Build a TemplateDocumentView for this document, which includes loading the document type slug,
    // tag slugs and cabinet slugs, as these may be needed to evaluate the document index templates.
    let document_view = get_document_view(&mut db, document_id)
        .await?;
    let template_document_view = build_template_document_view(&mut db, document_view).await?;

    // Check if the document index still exists - if not, we can skip this operation entirely.
    let document_index = document_indexes::table
        .find(document_index_id)
        .select(DocumentIndex::as_select())
        .first::<DocumentIndex>(&mut db)
        .await;
    if matches!(document_index, Err(diesel::result::Error::NotFound)) {
        tracing::info!(
            document_index_id,
            "document_index missing; skipping update"
        );
        return Ok(());
    }

    let document_view = Arc::new(template_document_view);

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
        .load::<i64>(&mut db)
        .await?;
    let mut existing_value_ids: HashSet<i64> = existing_value_ids.into_iter().collect();

    // Next, start at the root(s) of the document index (the template with no parent) and traverse down the tree, matching the document's metadata to the template's criteria, and updating the document_index_documents records as needed.
    // This function will evaluate the template against the DocumentView, using minijinja.
    let templates = document_index_templates::table
        .filter(document_index_templates::document_index_id.eq(document_index_id))
        .select(DocumentIndexTemplate::as_select())
        .load::<DocumentIndexTemplate>(&mut db)
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
    let templates = Arc::new(templates);
    let children_by_parent = Arc::new(children_by_parent);

    // Seed the stack with root templates (parent_id = None). Each root is processed independently
    // so a rollback only affects the current root.
    if let Some(root_ids) = children_by_parent.get(&None) {
        let root_ids: Vec<usize> = root_ids.clone();
        for root_idx in root_ids {
            let skip_due_to_empty_template = Arc::new(AtomicBool::new(false));
            let skip_due_to_empty_template_tx = Arc::clone(&skip_due_to_empty_template);
            let skip_due_to_no_leaf = Arc::new(AtomicBool::new(false));
            let skip_due_to_no_leaf_tx = Arc::clone(&skip_due_to_no_leaf);
            let templates = Arc::clone(&templates);
            let children_by_parent = Arc::clone(&children_by_parent);
            let document_view = Arc::clone(&document_view);

            let tx_result = db
                .build_transaction()
                .run::<Vec<i64>, diesel::result::Error, _>(|conn| {
                    Box::pin(async move {
                        let mut stack: Vec<(usize, Option<i64>)> = vec![(root_idx, None)];
                        let mut leaf_found = false;
                        let mut matched_leaf_value_ids: Vec<i64> = Vec::new();

                        // Depth-first traversal using an explicit stack to avoid recursive async calls.
                        while let Some((idx, parent_value_id)) = stack.pop() {
                            let template = &templates[idx];

                            let value_id = apply_document_index_value(
                                conn,
                                &document_view,
                                template,
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
                                matched_leaf_value_ids.push(value_id);
                            }
                        }

                        if !leaf_found {
                            skip_due_to_no_leaf_tx.store(true, Ordering::Relaxed);
                            return Err(diesel::result::Error::RollbackTransaction);
                        }

                        Ok(matched_leaf_value_ids)
                    })
                })
                .await;

            match tx_result {
                Ok(matched_leaf_value_ids) => {
                    for value_id in matched_leaf_value_ids {
                        existing_value_ids.remove(&value_id);
                    }
                }
                Err(diesel::result::Error::RollbackTransaction) => {
                    let should_skip =
                        skip_due_to_empty_template.as_ref().load(Ordering::Relaxed)
                            || skip_due_to_no_leaf.as_ref().load(Ordering::Relaxed);
                    if should_skip {
                        continue;
                    }
                    return Err(diesel::result::Error::RollbackTransaction.into());
                }
                Err(err) => {
                    return Err(err.into());
                }
            }
        }
    }

    let cleanup_result = db
        .build_transaction()
        .run::<_, diesel::result::Error, _>(|conn| {
            Box::pin(async move {
                // Any existing document_index_document records for this document that were not matched by the traversal
                // need to be removed, as they are no longer relevant.
                let stale_value_ids: Vec<i64> = existing_value_ids.iter().copied().collect();
                if stale_value_ids.is_empty() {
                    return Ok(());
                }

                // Next, delete the document_index_document records for these stale value ids, and collect the deleted value ids
                // to check if any document_index_value records need to be removed as well.
                let deleted_value_ids: Vec<i64> = diesel::delete(
                    document_index_documents::table.filter(
                        document_index_documents::document_id
                            .eq(document_id)
                            .and(document_index_documents::document_index_value_id.eq_any(&stale_value_ids)),
                    ),
                )
                .returning(document_index_documents::document_index_value_id)
                .get_results(conn)
                .await?;

                // Finally, delete any document_index_value records that are no longer linked to any document_index_documents
                // after removal, and their ancestor values if they no longer have children.
                delete_stale_document_index_values(conn, deleted_value_ids)
                    .await
                    .map_err(|err| {
                        tracing::error!(error = %err, "document_index_values cleanup failed inside transaction");
                        err
                    })?;
                Ok(())
            })
        })
        .await;

    cleanup_result?;

    Ok(())
}

pub async fn delete_document_index_document(
    db: &mut AsyncPgConnection,
    document_id: i64,
) -> Result<(), diesel::result::Error> {
    let deleted_value_ids: Vec<i64> = diesel::delete(
        document_index_documents::table
            .filter(document_index_documents::document_id.eq(document_id)),
    )
    .returning(document_index_documents::document_index_value_id)
    .get_results(db)
    .await?;
    if deleted_value_ids.is_empty() {
        return Ok(());
    }
    delete_stale_document_index_values(db, deleted_value_ids).await
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
) -> JobResult<TemplateDocumentView> {
    let document_type_slug = document_types::table
        .find(document_view.document_type_id)
        .select(document_types::slug)
        .first::<String>(db)
        .await?;

    let tags: HashSet<String> = if document_view.tag_ids.is_empty() {
        HashSet::new()
    } else {
        tags::table
            .filter(tags::id.eq_any(&document_view.tag_ids))
            .select(tags::slug)
            .load::<String>(db)
            .await?
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
            .await?
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
 * Internal function to delete document_index_value records that are no longer linked to any
 * document_index_documents after removal, and their ancestor values if they no longer have children.
 */
async fn delete_stale_document_index_values(
    db: &mut AsyncPgConnection,
    remaining_value_ids: Vec<i64>,
) -> Result<(), diesel::result::Error> {
    if remaining_value_ids.is_empty() {
        return Ok(());
    }

    // Delete any ancestor values that no longer have children after leaf removal.
    diesel::sql_query(
        r#"
        WITH RECURSIVE deletable AS (
            SELECT t.id, t.parent_id
            FROM document_index_values t
            WHERE t.id = ANY($1)
            AND NOT EXISTS (
                SELECT 1
                FROM document_index_documents d
                WHERE d.document_index_value_id = t.id
            )

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
        .bind::<diesel::sql_types::Array<diesel::sql_types::BigInt>, _>(remaining_value_ids)
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
            document_index_values::document_index_id.eq(template.document_index_id),
            document_index_values::document_index_template_id.eq(template.id),
            document_index_values::value.eq(rendered_value),
            document_index_values::parent_id.eq(parent_value_id),
            document_index_values::is_leaf.eq(template.is_leaf),
        ))
        .on_conflict((
            document_index_values::document_index_template_id,
            document_index_values::value,
        ))
        .do_update()
        .set((
            document_index_values::document_index_id.eq(diesel::upsert::excluded(document_index_values::document_index_id)),
            document_index_values::parent_id.eq(diesel::upsert::excluded(document_index_values::parent_id)),
            document_index_values::is_leaf.eq(diesel::upsert::excluded(document_index_values::is_leaf)),
        ))
        .returning(document_index_values::id)
        .get_result(db)
        .await?;

    // If this is a leaf node, we need upsert a document_index_documents record for this document and the document_index_value_id we just upserted.
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
    }

    Ok(Some(value_id))
}
