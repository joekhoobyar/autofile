use chrono::Utc;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::application::document_index_documents::delete_document_index_document;
use crate::application::jobs::{FastJob, MediumJob};
use crate::domain::documents::{Document, DocumentChangeset, DocumentView};
use crate::infrastructure::s3::delete_prefix_from_s3;
use crate::schema::{
    cabinet_documents, document_file_ocr_pages, document_file_pages, document_files,
    document_metadatas, document_types_metadata_types, documents, metadata_types, tag_documents,
};
use crate::shared::app_state::AppState;
use crate::shared::util::{ApiError, diesel_to_http};

use apalis::prelude::*;
use bb8::PooledConnection;
use diesel::dsl::{exists, not, sum};
use diesel::prelude::*;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::{AsyncPgConnection, RunQueryDsl};

pub async fn delete_document(
    state: Arc<AppState>,
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    id: i64,
) -> Result<(), ApiError> {
    let prefixes = db
        .build_transaction()
        .run::<_, diesel::result::Error, _>(|conn| {
            Box::pin(async move {
                // Remove document index associations before deleting the document row.
                delete_document_index_document(conn, id).await?;

                diesel::delete(
                    cabinet_documents::table.filter(cabinet_documents::document_id.eq(id)),
                )
                .execute(conn)
                .await?;

                diesel::delete(tag_documents::table.filter(tag_documents::document_id.eq(id)))
                    .execute(conn)
                    .await?;

                diesel::delete(
                    document_metadatas::table.filter(document_metadatas::document_id.eq(id)),
                )
                .execute(conn)
                .await?;

                diesel::delete(
                    document_file_ocr_pages::table.filter(exists(
                        document_files::table
                            .filter(document_files::document_id.eq(id))
                            .filter(
                                document_files::id.eq(document_file_ocr_pages::document_file_id),
                            ),
                    )),
                )
                .execute(conn)
                .await?;

                diesel::delete(
                    document_file_pages::table.filter(exists(
                        document_files::table
                            .filter(document_files::document_id.eq(id))
                            .filter(document_files::id.eq(document_file_pages::document_file_id)),
                    )),
                )
                .execute(conn)
                .await?;

                let prefixes: Vec<String> = diesel::delete(
                    document_files::table.filter(document_files::document_id.eq(id)),
                )
                .returning(document_files::s3_prefix)
                .get_results(conn)
                .await?;

                let affected = diesel::delete(documents::table.filter(documents::id.eq(id)))
                    .execute(conn)
                    .await?;
                if affected == 0 {
                    return Err(diesel::result::Error::NotFound);
                }

                Ok(prefixes)
            })
        })
        .await
        .map_err(|e| {
            if matches!(e, diesel::result::Error::NotFound) {
                ApiError::not_found("Document not found")
            } else {
                ApiError::new(diesel_to_http(e), "Failed to delete document")
            }
        })?;

    if !prefixes.is_empty() {
        let unique_prefixes: HashSet<String> = prefixes.into_iter().collect();
        for prefix in unique_prefixes {
            let delete_prefix = format!("{}/", prefix);
            delete_prefix_from_s3(&state.s3_client, &state.s3_bucket, &delete_prefix)
                .await
                .map_err(|e| {
                    ApiError::internal_server_error(&format!(
                        "Failed to delete document files from storage: {}",
                        e
                    ))
                })?;
        }
    }

    Ok(())
}

pub async fn enqueue_document_file_page_processing(
    state: Arc<AppState>,
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    document_id: i64,
) -> Result<(), ApiError> {
    let file_ids: Vec<i64> = document_files::table
        .filter(document_files::document_id.eq(document_id))
        .select(document_files::id)
        .order(document_files::id.asc())
        .load::<i64>(db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to list document files"))?;

    let mut medium_jobs = state.medium_jobs.as_ref().clone();
    for document_file_id in file_ids {
        medium_jobs
            .push(MediumJob::ProcessFilePages { document_file_id })
            .await
            .map_err(|_| ApiError::internal_server_error("Failed to enqueue file pages job"))?;
    }

    Ok(())
}

pub async fn enqueue_document_thumbnail_generation(
    state: Arc<AppState>,
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    document_id: i64,
) -> Result<(), ApiError> {
    let document_file_id = document_files::table
        .filter(document_files::document_id.eq(document_id))
        .select(document_files::id)
        .order(document_files::id.asc())
        .first::<i64>(db)
        .await
        .optional()
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to fetch document file"))?;

    let Some(document_file_id) = document_file_id else {
        return Ok(());
    };

    let mut fast_jobs = state.fast_jobs.as_ref().clone();
    fast_jobs
        .push(FastJob::GenerateThumbnail {
            document_file_id,
            page: 1,
            width: 800,
        })
        .await
        .map_err(|_| ApiError::internal_server_error("Failed to enqueue thumbnail job"))?;

    Ok(())
}

pub async fn enqueue_document_classification(
    state: Arc<AppState>,
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    document_id: i64,
    user_id: i64,
) -> Result<(), ApiError> {
    documents::table
        .find(document_id)
        .select(documents::id)
        .first::<i64>(db)
        .await
        .map_err(|e| {
            if matches!(e, diesel::result::Error::NotFound) {
                ApiError::not_found("Document not found")
            } else {
                ApiError::new(diesel_to_http(e), "Failed to fetch document")
            }
        })?;

    let mut medium_jobs = state.medium_jobs.as_ref().clone();
    medium_jobs
        .push(MediumJob::ClassifyDocument {
            document_id,
            user_id,
        })
        .await
        .map_err(|_| ApiError::internal_server_error("Failed to enqueue classify document job"))?;

    Ok(())
}

/**
 * This function retrieves a document by its ID and constructs a DocumentView,
 * which also includes retrieving the document's metadata, associated cabinets, and tags.
 */
pub async fn get_document_view(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    id: i64,
) -> Result<DocumentView, ApiError> {
    let document = documents::table
        .find(id)
        .select(Document::as_select())
        .first::<Document>(db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to fetch document"))?;

    let metadata_rows: Vec<(String, String)> = document_metadatas::table
        .inner_join(metadata_types::table)
        .filter(document_metadatas::document_id.eq(document.id))
        .select((metadata_types::slug, document_metadatas::value))
        .load::<(String, String)>(db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to list document metadata"))?;
    let metadata: HashMap<String, String> = metadata_rows.into_iter().collect();

    let cabinet_rows: Vec<i64> = cabinet_documents::table
        .filter(cabinet_documents::document_id.eq(document.id))
        .select(cabinet_documents::cabinet_id)
        .load::<i64>(db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to list cabinets for document"))?;
    let cabinet_ids: Vec<i64> = cabinet_rows.into_iter().collect();

    let tag_rows: Vec<i64> = tag_documents::table
        .filter(tag_documents::document_id.eq(document.id))
        .select(tag_documents::tag_id)
        .load::<i64>(db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to list tags for document"))?;
    let tag_ids: Vec<i64> = tag_rows.into_iter().collect();

    let pages_sum: Option<i64> = document_files::table
        .filter(document_files::document_id.eq(document.id))
        .select(sum(document_files::pages))
        .first::<Option<i64>>(db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to fetch document pages"))?;
    let pages = pages_sum.unwrap_or(0) as i32;

    Ok(DocumentView {
        id: document.id,
        title: document.title,
        document_type_id: document.document_type_id,
        pages,
        cabinet_ids,
        tag_ids,
        metadata,
        created_by: document.created_by,
        created_at: document.created_at,
        updated_by: document.updated_by,
        updated_at: document.updated_at,
    })
}

/**
 * This function updates a document's title and document type, and also ensures that
 * any metadata not allowed by the new document type is deleted.
 */
pub async fn update_document(
    user_id: i64,
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    id: i64,
    input: DocumentChangeset,
) -> Result<Document, ApiError> {
    let updated = db
        .build_transaction()
        .run::<_, diesel::result::Error, _>(|conn| {
            Box::pin(async move {
                // Update + return the updated row in one round-trip.
                let updated: Document =
                    diesel::update(documents::table.filter(documents::id.eq(id)))
                        .set((
                            &input,
                            documents::updated_by.eq(user_id),
                            documents::updated_at.eq(Utc::now()),
                        ))
                        .returning(Document::as_returning())
                        .get_result(conn)
                        .await?;

                // Determine the metadata types that are allowed by the document type.
                let allowed_metadata_link = document_types_metadata_types::table
                    .filter(
                        document_types_metadata_types::document_type_id
                            .eq(updated.document_type_id),
                    )
                    .filter(
                        document_types_metadata_types::metadata_type_id
                            .eq(document_metadatas::metadata_type_id),
                    );

                // Delete any metadata for this document that is not allowed by the document type.
                diesel::delete(
                    document_metadatas::table
                        .filter(document_metadatas::document_id.eq(id))
                        .filter(not(exists(allowed_metadata_link))),
                )
                .execute(conn)
                .await?;

                Ok(updated)
            })
        })
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to update document"))?;

    Ok(updated)
}
