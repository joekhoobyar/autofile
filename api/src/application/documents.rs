use chrono::Utc;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::application::classifier_blocks::{compute_classification_actions, load_document_text};
use crate::application::document_files::{
    BufferedDocumentFileUpload, cleanup_buffered_document_file_upload,
    delete_uploaded_document_file_from_s3, insert_document_file, upload_document_file_to_s3,
};
use crate::application::document_index_documents::build_template_document_view;
use crate::application::document_index_documents::delete_document_index_document;
use crate::application::document_index_documents::enqueue_document_index_document_updates;
use crate::application::jobs::{FastJob, MediumJob};
use crate::domain::classifier_blocks::ClassifierBlock;
use crate::domain::document_indexes::DocumentIndexValue;
use crate::domain::documents::{Document, DocumentChangeset, DocumentView};
use crate::infrastructure::s3::delete_prefix_from_s3;
use crate::schema::{
    cabinet_documents, classifier_blocks, document_file_ocr_pages, document_file_pages,
    document_files, document_index_documents, document_index_values, document_metadatas,
    document_types_metadata_types, documents, metadata_types, tag_documents,
};
use crate::shared::app_state::AppState;
use crate::shared::errors::{ApiError, ApiErrorContext, AppErrorContext, AppResult};
use crate::shared::responses::ResourceList;

use apalis::prelude::*;
use axum::http::StatusCode;
use bb8::PooledConnection;
use diesel::dsl::{exists, not, sum};
use diesel::prelude::*;
use diesel::sql_types::Bool;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use diesel_full_text_search::*;

pub struct CreateDocumentInput {
    pub title: Option<String>,
    pub document_type_id: Option<i64>,
    pub file_upload: Option<BufferedDocumentFileUpload>,
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct TestClassifierBlockResponse {
    pub computed_actions: HashMap<String, String>,
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct TestTemplateResponse {
    pub rendered: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, serde::Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum DocumentSortField {
    Id,
    Title,
    CreatedAt,
    UpdatedAt,
}

#[derive(Debug, serde::Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ListDocumentsQuery {
    /// 1-based page number.
    pub page: Option<i64>,
    /// Items per page (1 through 200).
    pub per_page: Option<i64>,
    /// When true, match any of the text-search criteria instead of all.
    pub match_any: Option<bool>,
    /// Case-insensitive title substring search.
    pub q: Option<String>,
    /// Full-text search over extracted document text and OCR text.
    pub text: Option<String>,
    /// Narrow results to one Document Type.
    pub document_type_id: Option<i64>,
    /// Narrow results to documents in one cabinet.
    pub cabinet_id: Option<i64>,
    /// Narrow results to documents with one tag.
    pub tag_id: Option<i64>,
    /// Narrow results to documents with a value for one Metadata Type.
    pub metadata_type_id: Option<i64>,
    /// Case-insensitive metadata value substring match.
    pub metadata_value: Option<String>,
    /// Case-insensitive filename substring search.
    pub filename: Option<String>,
    /// Narrow results to documents with a file content type substring match.
    pub file_content_type: Option<String>,
    /// Narrow results to documents assigned to one document index value.
    pub document_index_value_id: Option<i64>,
    /// When true, narrow results to documents sharing a title with another document.
    pub duplicates: Option<bool>,
    /// When true, narrow results to documents with a file checksum also present on another document.
    pub duplicate_checksum: Option<bool>,
    /// Sort field.
    pub sf: Option<DocumentSortField>,
    /// Set to true for descending order.
    pub sd: Option<bool>,
}

pub async fn create_document(
    state: Arc<AppState>,
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    user_id: i64,
    input: CreateDocumentInput,
) -> Result<Document, ApiError> {
    let CreateDocumentInput {
        title,
        document_type_id,
        mut file_upload,
    } = input;

    let title = match title {
        Some(value) => value,
        None => {
            if let Some(upload) = &file_upload {
                cleanup_buffered_document_file_upload(upload).await;
            }
            return Err(ApiError::bad_request("Missing required field: title"));
        }
    };
    let document_type_id = match document_type_id {
        Some(value) => value,
        None => {
            if let Some(upload) = &file_upload {
                cleanup_buffered_document_file_upload(upload).await;
            }
            return Err(ApiError::bad_request(
                "Missing required field: document_type_id",
            ));
        }
    };

    let mut file_info = None;
    if let Some(upload) = file_upload.take() {
        file_info = Some(upload_document_file_to_s3(&state, upload).await?);
    }

    let file_info_for_cleanup = file_info.clone();
    let fast_jobs = state.fast_jobs.as_ref().clone();
    let medium_jobs = state.medium_jobs.as_ref().clone();
    let thumb_enqueue_failed = Arc::new(AtomicBool::new(false));
    let thumb_enqueue_failed_for_tx = Arc::clone(&thumb_enqueue_failed);
    let pages_enqueue_failed = Arc::new(AtomicBool::new(false));
    let pages_enqueue_failed_for_tx = Arc::clone(&pages_enqueue_failed);

    let result = db
        .build_transaction()
        .run::<_, diesel::result::Error, _>(async move |conn| {
            let mut fast_jobs = fast_jobs;
            let mut medium_jobs = medium_jobs;
            let inserted_document: Document = diesel::insert_into(documents::table)
                .values((
                    documents::title.eq(&title),
                    documents::document_type_id.eq(document_type_id),
                    documents::created_by.eq(user_id),
                    documents::updated_by.eq(user_id),
                ))
                .returning(Document::as_returning())
                .get_result(conn)
                .await?;

            if let Some(upload) = file_info {
                let inserted_file =
                    insert_document_file(conn, inserted_document.id, upload, user_id).await?;

                if fast_jobs
                    .push(FastJob::GenerateThumbnail {
                        document_file_id: inserted_file.id,
                        page: 1,
                        width: 800,
                    })
                    .await
                    .is_err()
                {
                    thumb_enqueue_failed_for_tx.store(true, Ordering::Relaxed);
                    return Err(diesel::result::Error::RollbackTransaction);
                }

                if medium_jobs
                    .push(MediumJob::ProcessFilePages {
                        document_file_id: inserted_file.id,
                    })
                    .await
                    .is_err()
                {
                    pages_enqueue_failed_for_tx.store(true, Ordering::Relaxed);
                    return Err(diesel::result::Error::RollbackTransaction);
                }
            }

            Ok(inserted_document)
        })
        .await;

    match result {
        Ok(document) => {
            enqueue_document_index_document_updates(document.id, state.clone()).await?;
            Ok(document)
        }
        Err(e) => {
            if let Some(upload) = file_info_for_cleanup {
                delete_uploaded_document_file_from_s3(&state, &upload).await;
            }
            if matches!(e, diesel::result::Error::RollbackTransaction) {
                let thumb_failed = thumb_enqueue_failed.as_ref().load(Ordering::Relaxed);
                let pages_failed = pages_enqueue_failed.as_ref().load(Ordering::Relaxed);
                if thumb_failed && pages_failed {
                    Err(ApiError::new(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Failed to enqueue document processing jobs",
                    ))
                } else if thumb_failed {
                    Err(ApiError::new(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Failed to enqueue thumbnail job",
                    ))
                } else if pages_failed {
                    Err(ApiError::new(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Failed to enqueue file pages job",
                    ))
                } else {
                    Err(ApiError::from_diesel("Failed to create document", e))
                }
            } else {
                Err(ApiError::from_diesel("Failed to create document", e))
            }
        }
    }
}

pub async fn delete_document(
    state: Arc<AppState>,
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    id: i64,
) -> Result<(), ApiError> {
    let prefixes = db
        .build_transaction()
        .run::<_, diesel::result::Error, _>(async move |conn| {
            // Remove document index associations before deleting the document row.
            delete_document_index_document(conn, id).await?;

            diesel::delete(cabinet_documents::table.filter(cabinet_documents::document_id.eq(id)))
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
                        .filter(document_files::id.eq(document_file_ocr_pages::document_file_id)),
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

            let prefixes: Vec<String> =
                diesel::delete(document_files::table.filter(document_files::document_id.eq(id)))
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
        .await
        .map_err(|e| {
            if matches!(e, diesel::result::Error::NotFound) {
                ApiError::not_found("Document not found")
            } else {
                ApiError::from_diesel("Failed to delete document", e)
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
        .api_context("Failed to list document files")?;

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
        .api_context("Failed to fetch document file")?;

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
                ApiError::from_diesel("Failed to fetch document", e)
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

pub async fn list_documents(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    params: ListDocumentsQuery,
) -> Result<ResourceList<DocumentView>, ApiError> {
    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(50).clamp(1, 200);
    let offset = (page - 1) * per_page;
    let match_any = params.match_any.unwrap_or_default();

    let base_filter = || -> documents::BoxedQuery<'_, diesel::pg::Pg> {
        let mut query = documents::table.into_boxed();

        if let Some(q) = params.q.as_deref().filter(|s| !s.is_empty()) {
            let pattern = format!("%{}%", q);
            let criteria = documents::title.ilike(pattern);
            if match_any {
                query = query.or_filter(criteria);
            } else {
                query = query.filter(criteria);
            }
        }

        if let Some(text) = params.text.as_deref().filter(|s| !s.is_empty()) {
            let text_subquery = document_file_pages::table
                .inner_join(
                    document_files::table
                        .on(document_files::id.eq(document_file_pages::document_file_id)),
                )
                .filter(
                    document_file_pages::text_ts
                        .assume_not_null()
                        .matches(phraseto_tsquery(text)),
                )
                .filter(document_files::document_id.eq(documents::id));

            let ocr_subquery = document_file_ocr_pages::table
                .inner_join(
                    document_files::table
                        .on(document_files::id.eq(document_file_ocr_pages::document_file_id)),
                )
                .filter(
                    document_file_ocr_pages::ocr_ts
                        .assume_not_null()
                        .matches(phraseto_tsquery(text)),
                )
                .filter(document_files::document_id.eq(documents::id));

            if match_any {
                query = query
                    .or_filter(exists(text_subquery))
                    .or_filter(exists(ocr_subquery));
            } else {
                query = query.filter(exists(text_subquery).or(exists(ocr_subquery)));
            }
        }

        if let Some(filename) = params.filename.as_deref().filter(|s| !s.is_empty()) {
            let pattern = format!("%{}%", filename);
            let subquery = document_files::table
                .filter(document_files::document_id.eq(documents::id))
                .filter(document_files::filename.ilike(pattern));

            if match_any {
                query = query.or_filter(exists(subquery));
            } else {
                query = query.filter(exists(subquery));
            }
        }

        if let Some(id) = params.document_type_id {
            query = query.filter(documents::document_type_id.eq(id));
        }

        if let Some(value) = params.metadata_value.as_deref().filter(|s| !s.is_empty()) {
            let pattern = format!("%{}%", value);

            if let Some(metadata_type_id) = params.metadata_type_id {
                let subquery = document_metadatas::table
                    .filter(document_metadatas::document_id.eq(documents::id))
                    .filter(document_metadatas::value.ilike(pattern))
                    .filter(document_metadatas::metadata_type_id.eq(metadata_type_id));

                query = query.filter(exists(subquery));
            } else {
                let subquery = document_metadatas::table
                    .filter(document_metadatas::document_id.eq(documents::id))
                    .filter(document_metadatas::value.ilike(pattern));

                query = query.filter(exists(subquery));
            }
        } else if let Some(metadata_type_id) = params.metadata_type_id {
            let subquery = document_metadatas::table
                .filter(document_metadatas::document_id.eq(documents::id))
                .filter(document_metadatas::metadata_type_id.eq(metadata_type_id));

            query = query.filter(exists(subquery));
        }

        if let Some(content_type) = params
            .file_content_type
            .as_deref()
            .filter(|s| !s.is_empty())
        {
            let pattern = format!("%{}%", content_type);
            let subquery = document_files::table
                .filter(document_files::document_id.eq(documents::id))
                .filter(document_files::content_type.ilike(pattern));

            query = query.filter(exists(subquery));
        }

        if let Some(id) = params.cabinet_id {
            let subquery = cabinet_documents::table
                .filter(cabinet_documents::cabinet_id.eq(id))
                .filter(cabinet_documents::document_id.eq(documents::id));

            query = query.filter(exists(subquery));
        }

        if let Some(id) = params.tag_id {
            let subquery = tag_documents::table
                .filter(tag_documents::tag_id.eq(id))
                .filter(tag_documents::document_id.eq(documents::id));

            query = query.filter(exists(subquery));
        }

        if let Some(id) = params.document_index_value_id {
            let subquery = document_index_documents::table
                .filter(document_index_documents::document_index_value_id.eq(id))
                .filter(document_index_documents::document_id.eq(documents::id));

            query = query.filter(exists(subquery));
        }

        if params.duplicates.unwrap_or_default() {
            let duplicate_documents = diesel::alias!(documents as duplicate_documents);
            let subquery = duplicate_documents
                .filter(
                    duplicate_documents
                        .field(documents::title)
                        .eq(documents::title),
                )
                .filter(duplicate_documents.field(documents::id).ne(documents::id));

            query = query.filter(exists(subquery));
        }

        if params.duplicate_checksum.unwrap_or_default() {
            let criteria = diesel::dsl::sql::<Bool>(
                r#"
                EXISTS (
                    SELECT 1
                    FROM document_files matching_files
                    WHERE matching_files.document_id = documents.id
                      AND matching_files.checksum_sha256 IS NOT NULL
                      AND EXISTS (
                          SELECT 1
                          FROM document_files duplicate_files
                          WHERE duplicate_files.checksum_sha256 = matching_files.checksum_sha256
                            AND duplicate_files.document_id <> matching_files.document_id
                      )
                )
                "#,
            );

            query = query.filter(criteria);
        }

        query
    };

    let total = base_filter()
        .count()
        .get_result::<i64>(db)
        .await
        .api_context("Failed to count document_types")?;

    let mut query: documents::BoxedQuery<'_, diesel::pg::Pg> = base_filter();
    query = match (params.sf, params.sd) {
        (Some(DocumentSortField::Title), Some(true)) => {
            query.order((documents::title.desc(), documents::id.asc()))
        }
        (Some(DocumentSortField::Title), _) => {
            query.order((documents::title.asc(), documents::id.asc()))
        }
        (Some(DocumentSortField::CreatedAt), Some(true)) => {
            query.order((documents::created_at.desc(), documents::id.asc()))
        }
        (Some(DocumentSortField::CreatedAt), _) => {
            query.order((documents::created_at.asc(), documents::id.asc()))
        }
        (Some(DocumentSortField::UpdatedAt), Some(true)) => {
            query.order((documents::updated_at.desc(), documents::id.asc()))
        }
        (Some(DocumentSortField::UpdatedAt), _) => {
            query.order((documents::updated_at.asc(), documents::id.asc()))
        }
        (Some(DocumentSortField::Id), Some(true)) => query.order(documents::id.desc()),
        _ => query.order(documents::id.asc()),
    };

    let documents = query
        .limit(per_page)
        .offset(offset)
        .select(Document::as_select())
        .load::<Document>(db)
        .await
        .api_context("Failed to list documents")?;
    let document_ids: Vec<i64> = documents.iter().map(|doc| doc.id).collect();

    let mut pages_by_document: HashMap<i64, i32> = HashMap::new();
    if !document_ids.is_empty() {
        let pages_rows: Vec<(i64, Option<i64>)> = document_files::table
            .filter(document_files::document_id.eq_any(&document_ids))
            .group_by(document_files::document_id)
            .select((document_files::document_id, sum(document_files::pages)))
            .load::<(i64, Option<i64>)>(db)
            .await
            .api_context("Failed to list document pages")?;

        for (document_id, pages_sum) in pages_rows {
            let pages = pages_sum.unwrap_or(0) as i32;
            pages_by_document.insert(document_id, pages);
        }
    }

    let mut metadata_by_document: HashMap<i64, HashMap<String, String>> = HashMap::new();
    if !document_ids.is_empty() {
        let metadata_rows: Vec<(i64, String, String)> = document_metadatas::table
            .inner_join(metadata_types::table)
            .filter(document_metadatas::document_id.eq_any(&document_ids))
            .select((
                document_metadatas::document_id,
                metadata_types::slug,
                document_metadatas::value,
            ))
            .load::<(i64, String, String)>(db)
            .await
            .api_context("Failed to list document metadata")?;

        for (document_id, slug, value) in metadata_rows {
            metadata_by_document
                .entry(document_id)
                .or_default()
                .insert(slug, value);
        }
    }

    let mut cabinets_by_document: HashMap<i64, Vec<i64>> = HashMap::new();
    if !document_ids.is_empty() {
        let cabinet_rows: Vec<(i64, i64)> = cabinet_documents::table
            .filter(cabinet_documents::document_id.eq_any(&document_ids))
            .select((
                cabinet_documents::document_id,
                cabinet_documents::cabinet_id,
            ))
            .load::<(i64, i64)>(db)
            .await
            .map_err(|e| ApiError::from_diesel("Failed to list cabinets for documents", e))?;

        for (document_id, cabinet_id) in cabinet_rows {
            cabinets_by_document
                .entry(document_id)
                .or_default()
                .push(cabinet_id);
        }
    }

    let mut tags_by_document: HashMap<i64, Vec<i64>> = HashMap::new();
    if !document_ids.is_empty() {
        let tag_rows: Vec<(i64, i64)> = tag_documents::table
            .filter(tag_documents::document_id.eq_any(&document_ids))
            .select((tag_documents::document_id, tag_documents::tag_id))
            .load::<(i64, i64)>(db)
            .await
            .api_context("Failed to list tags for documents")?;

        for (document_id, tag_id) in tag_rows {
            tags_by_document
                .entry(document_id)
                .or_default()
                .push(tag_id);
        }
    }

    let items = documents
        .into_iter()
        .map(|doc| DocumentView {
            id: doc.id,
            title: doc.title,
            document_type_id: doc.document_type_id,
            pages: pages_by_document.remove(&doc.id).unwrap_or(0),
            metadata: metadata_by_document.remove(&doc.id).unwrap_or_default(),
            cabinet_ids: cabinets_by_document.remove(&doc.id).unwrap_or_default(),
            tag_ids: tags_by_document.remove(&doc.id).unwrap_or_default(),
            created_by: doc.created_by,
            created_at: doc.created_at,
            updated_by: doc.updated_by,
            updated_at: doc.updated_at,
        })
        .collect();

    Ok(ResourceList {
        total,
        page,
        per_page,
        items,
    })
}

pub async fn get_document_thumbnail_metadata(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    id: i64,
) -> Result<(String, chrono::DateTime<Utc>), ApiError> {
    let (s3_thumbnail, updated_at) = documents::table
        .find(id)
        .select((documents::s3_thumbnail, documents::updated_at))
        .first::<(Option<String>, chrono::DateTime<Utc>)>(db)
        .await
        .api_context("Failed to fetch document thumbnail")?;

    let s3_key = s3_thumbnail.ok_or_else(|| ApiError::not_found("Thumbnail not available"))?;

    Ok((s3_key, updated_at))
}

pub async fn list_document_index_values(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    id: i64,
) -> Result<Vec<DocumentIndexValue>, ApiError> {
    document_index_documents::table
        .inner_join(document_index_values::table)
        .filter(document_index_documents::document_id.eq(id))
        .select(DocumentIndexValue::as_select())
        .order(document_index_values::id.asc())
        .load::<DocumentIndexValue>(db)
        .await
        .api_context("Failed to list document_index_values")
}

pub async fn test_classifier_block(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    document_id: i64,
    classifier_block_id: i64,
) -> Result<TestClassifierBlockResponse, ApiError> {
    let classifier_block = classifier_blocks::table
        .find(classifier_block_id)
        .select(ClassifierBlock::as_select())
        .first::<ClassifierBlock>(db)
        .await
        .map_err(|e| {
            if matches!(e, diesel::result::Error::NotFound) {
                ApiError::not_found("Classifier block not found")
            } else {
                ApiError::from_diesel("Failed to fetch classifier_block", e)
            }
        })?;

    let document_view = get_document_view(db, document_id).await?;
    let document_text = load_document_text(db, document_id).await.map_err(|e| {
        ApiError::internal_server_error(&format!("Failed to load document text: {}", e))
    })?;

    let computed_actions = compute_classification_actions(
        document_id,
        &document_view,
        &document_text,
        std::slice::from_ref(&classifier_block),
    )
    .map_err(|e| {
        ApiError::internal_server_error(&format!("Failed to compute classification actions: {}", e))
    })?;

    Ok(TestClassifierBlockResponse { computed_actions })
}

pub async fn test_template(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    document_id: i64,
    template: &str,
) -> Result<TestTemplateResponse, ApiError> {
    let document_view = get_document_view(db, document_id).await?;
    let template_document_view = build_template_document_view(db, document_view)
        .await
        .map_err(|e| {
            ApiError::internal_server_error(&format!(
                "Failed to build template document view: {}",
                e
            ))
        })?;

    let env = minijinja::Environment::new();
    match env.render_str(
        template,
        minijinja::context! { doc => &template_document_view },
    ) {
        Ok(rendered) => Ok(TestTemplateResponse {
            rendered: Some(rendered),
            error: None,
        }),
        Err(err) => Ok(TestTemplateResponse {
            rendered: None,
            error: Some(err.to_string()),
        }),
    }
}

/**
 * This function retrieves a document by its ID and constructs a DocumentView,
 * which also includes retrieving the document's metadata, associated cabinets, and tags.
 */
pub async fn get_document_view(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    id: i64,
) -> AppResult<DocumentView> {
    let document = documents::table
        .find(id)
        .select(Document::as_select())
        .first::<Document>(db)
        .await
        .app_context("Failed to fetch document")?;

    let metadata_rows: Vec<(String, String)> = document_metadatas::table
        .inner_join(metadata_types::table)
        .filter(document_metadatas::document_id.eq(document.id))
        .select((metadata_types::slug, document_metadatas::value))
        .load::<(String, String)>(db)
        .await
        .app_context("Failed to list document metadata")?;
    let metadata: HashMap<String, String> = metadata_rows.into_iter().collect();

    let cabinet_rows: Vec<i64> = cabinet_documents::table
        .filter(cabinet_documents::document_id.eq(document.id))
        .select(cabinet_documents::cabinet_id)
        .load::<i64>(db)
        .await
        .app_context("Failed to list cabinets for document")?;
    let cabinet_ids: Vec<i64> = cabinet_rows.into_iter().collect();

    let tag_rows: Vec<i64> = tag_documents::table
        .filter(tag_documents::document_id.eq(document.id))
        .select(tag_documents::tag_id)
        .load::<i64>(db)
        .await
        .app_context("Failed to list tags for document")?;
    let tag_ids: Vec<i64> = tag_rows.into_iter().collect();

    let pages_sum: Option<i64> = document_files::table
        .filter(document_files::document_id.eq(document.id))
        .select(sum(document_files::pages))
        .first::<Option<i64>>(db)
        .await
        .app_context("Failed to fetch document pages")?;
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
        .run::<_, diesel::result::Error, _>(async move |conn| {
            // Update + return the updated row in one round-trip.
            let updated: Document = diesel::update(documents::table.filter(documents::id.eq(id)))
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
                    document_types_metadata_types::document_type_id.eq(updated.document_type_id),
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
        .await
        .api_context("Failed to update document")?;

    Ok(updated)
}
