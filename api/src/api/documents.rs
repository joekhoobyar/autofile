use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::application::classifier_blocks::{compute_classification_actions, load_document_text};
use crate::application::document_files::{
    BufferedDocumentFileUpload, UploadedDocumentFile, buffer_document_file_field,
    cleanup_buffered_document_file_upload, delete_uploaded_document_file_from_s3,
    insert_document_file, upload_document_file_to_s3,
};
use crate::application::document_index_documents::build_template_document_view;
use crate::application::document_index_documents::delete_document_index_document;
use crate::application::document_index_documents::enqueue_document_index_document_updates;
use crate::application::documents::{get_document_view, update_document};
use crate::application::jobs::{FastJob, MediumJob};
use crate::domain::classifier_blocks::ClassifierBlock;
use crate::domain::document_indexes::DocumentIndexValue;
use crate::domain::documents::{Document, DocumentChangeset, DocumentView};
use crate::infrastructure::s3::delete_prefix_from_s3;
use crate::schema::{
    cabinet_documents, classifier_blocks, document_file_ocr_pages, document_file_pages,
    document_files, document_index_documents, document_index_values, document_metadatas, documents,
    metadata_types, tag_documents,
};
use crate::shared::app_state::AppState;
use crate::shared::auth::AuthUser;
use crate::shared::extractors::DbConn;
use crate::shared::s3::serve_s3_file;
use crate::shared::util::{ApiError, ResourceList, diesel_to_http};

use axum::extract::DefaultBodyLimit;
use diesel::dsl::{exists, sum};
use diesel_full_text_search::*;
use serde::Deserialize;

use axum::{
    Json, Router,
    extract::{Multipart, Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::Response,
    routing::{get, post},
};
use diesel::prelude::*;
use diesel_async::{AsyncConnection, RunQueryDsl};

use apalis::prelude::*;

#[derive(Debug, Deserialize, Insertable)]
#[diesel(table_name = documents)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct NewDocument {
    title: String,
    document_type_id: i64,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentSortField {
    Id,
    Title,
    CreatedAt,
    UpdatedAt,
}

#[derive(Debug, Deserialize)]
pub struct ListDocumentsQuery {
    // 1-based page number
    pub page: Option<i64>,
    // items per page (cap it)
    pub per_page: Option<i64>,
    // if true, match any of the search criteria instead of all.
    pub match_any: Option<bool>,
    // optional substring search
    pub q: Option<String>,
    // optional text search
    pub text: Option<String>,
    // optional document type search
    pub document_type_id: Option<i64>,
    // optional cabinet search
    pub cabinet_id: Option<i64>,
    // optional tag search
    pub tag_id: Option<i64>,
    // optional metadata type search
    pub metadata_type_id: Option<i64>,
    // optional metadata value search
    pub metadata_value: Option<String>,
    // optional document index value search
    pub document_index_value_id: Option<i64>,
    // optional sort field
    pub sf: Option<DocumentSortField>,
    // optional sort descending
    pub sd: Option<bool>,
}

struct ParsedMultipart {
    title: Option<String>,
    document_type_id: Option<i64>,
    file_temp: Option<BufferedDocumentFileUpload>,
}

#[derive(Debug, Deserialize)]
struct TestClassifierBlockInput {
    classifier_block_id: i64,
}

#[derive(Debug, Deserialize)]
struct TestTemplateInput {
    template: String,
}

#[derive(Debug, serde::Serialize)]
struct TestClassifierBlockResponse {
    computed_actions: HashMap<String, String>,
}

#[derive(Debug, serde::Serialize)]
struct TestTemplateResponse {
    rendered: Option<String>,
    error: Option<String>,
}

async fn parse_create_multipart(multipart: &mut Multipart) -> Result<ParsedMultipart, ApiError> {
    let mut title: Option<String> = None;
    let mut document_type_id: Option<i64> = None;
    let mut file_temp: Option<BufferedDocumentFileUpload> = None;

    while let Some(mut field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::bad_request(&format!("Failed to read multipart field: {}", e)))?
    {
        let field_name = field
            .name()
            .ok_or_else(|| ApiError::bad_request("Field missing name"))?
            .to_string();

        match field_name.as_str() {
            "title" => {
                let value = field
                    .text()
                    .await
                    .map_err(|e| ApiError::bad_request(&format!("Failed to read title: {}", e)))?;
                title = Some(value);
            }
            "document_type_id" => {
                let value = field.text().await.map_err(|e| {
                    ApiError::bad_request(&format!("Failed to read document_type_id: {}", e))
                })?;
                document_type_id = Some(
                    value
                        .parse::<i64>()
                        .map_err(|_| ApiError::bad_request("Invalid document_type_id"))?,
                );
            }
            "file" => {
                if let Some(upload) = &file_temp {
                    cleanup_buffered_document_file_upload(upload).await;
                    return Err(ApiError::bad_request("Only one file upload is supported"));
                }
                file_temp = Some(buffer_document_file_field(&mut field).await?);
            }
            _ => {
                // Ignore unknown fields
            }
        }
    }

    Ok(ParsedMultipart {
        title,
        document_type_id,
        file_temp,
    })
}

pub async fn get_by_id(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Path(id): Path<i64>,
) -> Result<Json<DocumentView>, ApiError> {
    let document_view = get_document_view(&mut db, id).await?;
    Ok(Json(document_view))
}

pub async fn delete(
    _user: AuthUser,
    State(state): State<Arc<AppState>>,
    DbConn(mut db): DbConn,
    Path(id): Path<i64>,
) -> Result<Json<()>, ApiError> {
    let prefixes = db
        .transaction::<_, diesel::result::Error, _>(move |conn| {
            Box::pin(async move {
                // Remove document index associations before deleting the document row.
                delete_document_index_document(conn, id).await?;

                // Delete the cabinet document associations
                diesel::delete(
                    cabinet_documents::table.filter(cabinet_documents::document_id.eq(id)),
                )
                .execute(conn)
                .await?;

                // Delete the tag document associations
                diesel::delete(tag_documents::table.filter(tag_documents::document_id.eq(id)))
                    .execute(conn)
                    .await?;

                // Delete the document metadata associations
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

                // Delete the document files, fetching the S3 keys for later deletion.
                let prefixes: Vec<String> = diesel::delete(
                    document_files::table.filter(document_files::document_id.eq(id)),
                )
                .returning(document_files::s3_prefix)
                .get_results(conn)
                .await?;

                // Delete the document.
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

    Ok(Json(()))
}

pub async fn process_file_pages(
    _user: AuthUser,
    State(state): State<Arc<AppState>>,
    DbConn(mut db): DbConn,
    Path(id): Path<i64>,
) -> Result<Json<()>, ApiError> {
    let file_ids: Vec<i64> = document_files::table
        .filter(document_files::document_id.eq(id))
        .select(document_files::id)
        .order(document_files::id.asc())
        .load::<i64>(&mut db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to list document files"))?;

    if file_ids.is_empty() {
        return Ok(Json(()));
    }

    let mut medium_jobs = state.medium_jobs.as_ref().clone();
    for document_file_id in file_ids {
        if let Err(_) = medium_jobs
            .push(MediumJob::ProcessFilePages { document_file_id })
            .await
        {
            return Err(ApiError::internal_server_error(
                "Failed to enqueue file pages job",
            ));
        }
    }

    Ok(Json(()))
}

pub async fn generate_thumbnail(
    _user: AuthUser,
    State(state): State<Arc<AppState>>,
    DbConn(mut db): DbConn,
    Path(id): Path<i64>,
) -> Result<Json<()>, ApiError> {
    let document_file_id = document_files::table
        .filter(document_files::document_id.eq(id))
        .select(document_files::id)
        .order(document_files::id.asc())
        .first::<i64>(&mut db)
        .await
        .optional()
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to fetch document file"))?;

    let Some(document_file_id) = document_file_id else {
        return Ok(Json(()));
    };

    let mut fast_jobs = state.fast_jobs.as_ref().clone();
    if let Err(_) = fast_jobs
        .push(FastJob::GenerateThumbnail {
            document_file_id,
            page: 1,
            width: 800,
        })
        .await
    {
        return Err(ApiError::internal_server_error(
            "Failed to enqueue thumbnail job",
        ));
    }

    Ok(Json(()))
}

pub async fn classify_document(
    user: AuthUser,
    State(state): State<Arc<AppState>>,
    DbConn(mut db): DbConn,
    Path(id): Path<i64>,
) -> Result<Json<()>, ApiError> {
    documents::table
        .find(id)
        .select(documents::id)
        .first::<i64>(&mut db)
        .await
        .map_err(|e| {
            if matches!(e, diesel::result::Error::NotFound) {
                ApiError::not_found("Document not found")
            } else {
                ApiError::new(diesel_to_http(e), "Failed to fetch document")
            }
        })?;

    let mut medium_jobs = state.medium_jobs.as_ref().clone();
    if let Err(_) = medium_jobs
        .push(MediumJob::ClassifyDocument {
            document_id: id,
            user_id: user.user_id,
        })
        .await
    {
        return Err(ApiError::internal_server_error(
            "Failed to enqueue classify document job",
        ));
    }

    Ok(Json(()))
}

async fn test_classifier_block(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Path(id): Path<i64>,
    Json(input): Json<TestClassifierBlockInput>,
) -> Result<Json<TestClassifierBlockResponse>, ApiError> {
    let classifier_block = classifier_blocks::table
        .find(input.classifier_block_id)
        .select(ClassifierBlock::as_select())
        .first::<ClassifierBlock>(&mut db)
        .await
        .map_err(|e| {
            if matches!(e, diesel::result::Error::NotFound) {
                ApiError::not_found("Classifier block not found")
            } else {
                ApiError::new(diesel_to_http(e), "Failed to fetch classifier_block")
            }
        })?;

    let document_view = get_document_view(&mut db, id).await?;
    let document_text = load_document_text(&mut db, id).await.map_err(|e| {
        ApiError::internal_server_error(&format!("Failed to load document text: {}", e))
    })?;

    let computed_actions = compute_classification_actions(
        id,
        &document_view,
        &document_text,
        std::slice::from_ref(&classifier_block),
    )
    .map_err(|e| {
        ApiError::internal_server_error(&format!("Failed to compute classification actions: {}", e))
    })?;

    Ok(Json(TestClassifierBlockResponse { computed_actions }))
}

async fn test_template(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Path(id): Path<i64>,
    Json(input): Json<TestTemplateInput>,
) -> Result<Json<TestTemplateResponse>, ApiError> {
    let document_view = get_document_view(&mut db, id).await?;
    let template_document_view = build_template_document_view(&mut db, document_view)
        .await
        .map_err(|e| {
            ApiError::internal_server_error(&format!(
                "Failed to build template document view: {}",
                e
            ))
        })?;

    let env = minijinja::Environment::new();
    match env.render_str(
        &input.template,
        minijinja::context! { doc => &template_document_view },
    ) {
        Ok(rendered) => Ok(Json(TestTemplateResponse {
            rendered: Some(rendered),
            error: None,
        })),
        Err(err) => Ok(Json(TestTemplateResponse {
            rendered: None,
            error: Some(err.to_string()),
        })),
    }
}

/**
 * This handler serves the thumbnail image for a document, streaming it directly from S3.
 * It supports conditional GET with If-Modified-Since header to optimize caching.
 */
pub async fn thumbnail_get(
    _user: AuthUser,
    State(state): State<Arc<AppState>>,
    DbConn(mut db): DbConn,
    Path(id): Path<i64>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let (s3_thumbnail, updated_at) = documents::table
        .find(id)
        .select((documents::s3_thumbnail, documents::updated_at))
        .first::<(Option<String>, chrono::DateTime<chrono::Utc>)>(&mut db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to fetch document thumbnail"))?;

    let s3_key = s3_thumbnail.ok_or_else(|| ApiError::not_found("Thumbnail not available"))?;

    serve_s3_file(
        state.as_ref(),
        &headers,
        &s3_key,
        Some(updated_at),
        "Thumbnail not available",
        Some("image/png"),
    )
    .await
}

async fn create(
    user: AuthUser,
    State(state): State<Arc<AppState>>,
    DbConn(mut db): DbConn,
    mut multipart: Multipart,
) -> Result<Json<Document>, ApiError> {
    let ParsedMultipart {
        title,
        document_type_id,
        mut file_temp,
    } = parse_create_multipart(&mut multipart).await?;

    let mut file_info: Option<UploadedDocumentFile> = None;

    // Validate required fields
    let title = match title {
        Some(value) => value,
        None => {
            if let Some(upload) = &file_temp {
                cleanup_buffered_document_file_upload(upload).await;
            }
            return Err(ApiError::bad_request("Missing required field: title"));
        }
    };
    let document_type_id = match document_type_id {
        Some(value) => value,
        None => {
            if let Some(upload) = &file_temp {
                cleanup_buffered_document_file_upload(upload).await;
            }
            return Err(ApiError::bad_request(
                "Missing required field: document_type_id",
            ));
        }
    };

    // Upload the temp file to S3, then delete it.
    if let Some(upload) = file_temp.take() {
        file_info = Some(upload_document_file_to_s3(&state, upload).await?);
    }

    // Clone file_info for potential cleanup in error path
    let file_info_for_cleanup = file_info.clone();

    // Clone the fast job queue handle for use inside the transaction closure
    let fast_jobs = state.fast_jobs.as_ref().clone();
    let medium_jobs = state.medium_jobs.as_ref().clone();
    let thumb_enqueue_failed = Arc::new(AtomicBool::new(false));
    let thumb_enqueue_failed_for_tx = Arc::clone(&thumb_enqueue_failed);
    let pages_enqueue_failed = Arc::new(AtomicBool::new(false));
    let pages_enqueue_failed_for_tx = Arc::clone(&pages_enqueue_failed);

    // Begin database transaction
    let result = db
        .build_transaction()
        .run::<_, diesel::result::Error, _>(|conn| {
            Box::pin(async move {
                let mut fast_jobs = fast_jobs;
                let mut medium_jobs = medium_jobs;
                // Insert document record
                let inserted_document: Document = diesel::insert_into(documents::table)
                    .values((
                        documents::title.eq(&title),
                        documents::document_type_id.eq(document_type_id),
                        documents::created_by.eq(user.user_id),
                        documents::updated_by.eq(user.user_id),
                    ))
                    .returning(Document::as_returning())
                    .get_result(conn)
                    .await?;

                // If file was uploaded, insert document_files record
                if let Some(upload) = file_info {
                    let inserted_file =
                        insert_document_file(conn, inserted_document.id, upload, user.user_id)
                            .await?;

                    if let Err(_) = fast_jobs
                        .push(FastJob::GenerateThumbnail {
                            document_file_id: inserted_file.id,
                            page: 1,
                            width: 800,
                        })
                        .await
                    {
                        thumb_enqueue_failed_for_tx.store(true, Ordering::Relaxed);
                        return Err(diesel::result::Error::RollbackTransaction);
                    }

                    if let Err(_) = medium_jobs
                        .push(MediumJob::ProcessFilePages {
                            document_file_id: inserted_file.id,
                        })
                        .await
                    {
                        pages_enqueue_failed_for_tx.store(true, Ordering::Relaxed);
                        return Err(diesel::result::Error::RollbackTransaction);
                    }
                }

                Ok(inserted_document)
            })
        })
        .await;

    match result {
        Ok(document) => {
            // Enqueue jobs to update document indexes for this document, as the tags may be used in index rules.
            enqueue_document_index_document_updates(document.id, state.clone()).await?;

            Ok(Json(document))
        }
        Err(e) => {
            // On transaction failure, attempt S3 cleanup (best-effort)
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
                    Err(ApiError::new(
                        diesel_to_http(e),
                        "Failed to create document",
                    ))
                }
            } else {
                Err(ApiError::new(
                    diesel_to_http(e),
                    "Failed to create document",
                ))
            }
        }
    }
}

async fn update(
    user: AuthUser,
    State(state): State<Arc<AppState>>,
    DbConn(mut db): DbConn,
    Path(id): Path<i64>,
    Json(input): Json<DocumentChangeset>,
) -> Result<Json<Document>, ApiError> {
    let updated = update_document(user.user_id, &mut db, id, input).await?;

    // Enqueue jobs to update document indexes for this document, as the tags may be used in index rules.
    enqueue_document_index_document_updates(id, state).await?;

    Ok(Json(updated))
}

pub async fn list(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Query(params): Query<ListDocumentsQuery>,
) -> Result<Json<ResourceList<DocumentView>>, ApiError> {
    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(50).clamp(1, 200);
    let offset = (page - 1) * per_page;
    let match_any = params.match_any.unwrap_or_default();

    let base_filter = || -> documents::BoxedQuery<'_, diesel::pg::Pg> {
        // Start with a boxed query so we can conditionally add filters.
        let mut query = documents::table.into_boxed();

        // Optional search: case-insensitive substring on title
        if let Some(q) = params.q.as_deref().filter(|s| !s.is_empty()) {
            let pattern = format!("%{}%", q);
            let criteria = documents::title.ilike(pattern);
            if match_any {
                query = query.or_filter(criteria);
            } else {
                query = query.filter(criteria);
            }
        }

        // Filter by document type
        if let Some(id) = params.document_type_id {
            let criteria = documents::document_type_id.eq(id);
            if match_any {
                query = query.or_filter(criteria);
            } else {
                query = query.filter(criteria);
            }
        }

        // Filter by metadata type and value
        if let Some(value) = params.metadata_value.as_deref().filter(|s| !s.is_empty()) {
            let pattern = format!("%{}%", value);

            // When metadata_type_id is provided, filter by both value and type
            if let Some(metadata_type_id) = params.metadata_type_id {
                let subquery = document_metadatas::table
                    .filter(document_metadatas::document_id.eq(documents::id))
                    .filter(document_metadatas::value.ilike(pattern))
                    .filter(document_metadatas::metadata_type_id.eq(metadata_type_id));

                if match_any {
                    query = query.or_filter(exists(subquery));
                } else {
                    query = query.filter(exists(subquery));
                }
            } else {
                // When only value is provided (no metadata_type_id), filter by value only
                let subquery = document_metadatas::table
                    .filter(document_metadatas::document_id.eq(documents::id))
                    .filter(document_metadatas::value.ilike(pattern));

                if match_any {
                    query = query.or_filter(exists(subquery));
                } else {
                    query = query.filter(exists(subquery));
                }
            }
        }

        // Filter by document text
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

        // Filter by cabinet ID
        if let Some(id) = params.cabinet_id {
            let subquery = cabinet_documents::table
                .filter(cabinet_documents::cabinet_id.eq(id))
                .filter(cabinet_documents::document_id.eq(documents::id));

            if match_any {
                query = query.or_filter(exists(subquery));
            } else {
                query = query.filter(exists(subquery));
            }
        }

        // Filter by tag ID
        if let Some(id) = params.tag_id {
            let subquery = tag_documents::table
                .filter(tag_documents::tag_id.eq(id))
                .filter(tag_documents::document_id.eq(documents::id));

            if match_any {
                query = query.or_filter(exists(subquery));
            } else {
                query = query.filter(exists(subquery));
            }
        }

        // Filter by document index value ID
        if let Some(id) = params.document_index_value_id {
            let subquery = document_index_documents::table
                .filter(document_index_documents::document_index_value_id.eq(id))
                .filter(document_index_documents::document_id.eq(documents::id));

            if match_any {
                query = query.or_filter(exists(subquery));
            } else {
                query = query.filter(exists(subquery));
            }
        }

        query
    };

    // Count the total number of documents matching the filters (for pagination metadata)
    let total = base_filter()
        .count()
        .get_result::<i64>(&mut db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to count document_types"))?;

    // Apply sorting based on query parameters, with tie-breaker on ID for consistent pagination.
    let mut query: documents::BoxedQuery<'_, diesel::pg::Pg> = base_filter();
    query = match (params.sf, params.sd) {
        (Some(DocumentSortField::Title), Some(true)) => {
            query.order((documents::title.desc(), documents::id.asc()))
        } // tie-breaker
        (Some(DocumentSortField::Title), _) => {
            query.order((documents::title.asc(), documents::id.asc()))
        } // tie-breaker
        (Some(DocumentSortField::CreatedAt), Some(true)) => {
            query.order((documents::created_at.desc(), documents::id.asc()))
        } // tie-breaker
        (Some(DocumentSortField::CreatedAt), _) => {
            query.order((documents::created_at.asc(), documents::id.asc()))
        } // tie-breaker
        (Some(DocumentSortField::UpdatedAt), Some(true)) => {
            query.order((documents::updated_at.desc(), documents::id.asc()))
        } // tie-breaker
        (Some(DocumentSortField::UpdatedAt), _) => {
            query.order((documents::updated_at.asc(), documents::id.asc()))
        } // tie-breaker

        (Some(DocumentSortField::Id), Some(true)) => query.order(documents::id.desc()),
        _ => query.order(documents::id.asc()),
    };

    // Fetch the requested page of documents, then collect IDs for batch fetching of metadata and cabinets.
    let documents = query
        .limit(per_page)
        .offset(offset)
        .select(Document::as_select())
        .load::<Document>(&mut db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to list documents"))?;
    let document_ids: Vec<i64> = documents.iter().map(|doc| doc.id).collect();

    // Fetch pages per document for all documents in the page in a single query.
    let mut pages_by_document: HashMap<i64, i32> = HashMap::new();
    if !document_ids.is_empty() {
        let pages_rows: Vec<(i64, Option<i64>)> = document_files::table
            .filter(document_files::document_id.eq_any(&document_ids))
            .group_by(document_files::document_id)
            .select((document_files::document_id, sum(document_files::pages)))
            .load::<(i64, Option<i64>)>(&mut db)
            .await
            .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to list document pages"))?;

        for (document_id, pages_sum) in pages_rows {
            let pages = pages_sum.unwrap_or(0) as i32;
            pages_by_document.insert(document_id, pages);
        }
    }

    // Fetch metadata for all documents in the page in a single query, and organize it by document ID.
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
            .load::<(i64, String, String)>(&mut db)
            .await
            .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to list document metadata"))?;

        for (document_id, slug, value) in metadata_rows {
            metadata_by_document
                .entry(document_id)
                .or_insert_with(HashMap::new)
                .insert(slug, value);
        }
    }

    // Fetch cabinets for all documents in the page in a single query, and organize it by cabinet ID.
    let mut cabinets_by_document: HashMap<i64, Vec<i64>> = HashMap::new();
    if !document_ids.is_empty() {
        let cabinet_rows: Vec<(i64, i64)> = cabinet_documents::table
            .filter(cabinet_documents::document_id.eq_any(&document_ids))
            .select((
                cabinet_documents::document_id,
                cabinet_documents::cabinet_id,
            ))
            .load::<(i64, i64)>(&mut db)
            .await
            .map_err(|e| {
                ApiError::new(diesel_to_http(e), "Failed to list cabinets for documents")
            })?;

        for (document_id, cabinet_id) in cabinet_rows {
            cabinets_by_document
                .entry(document_id)
                .or_insert_with(Vec::new)
                .push(cabinet_id);
        }
    }

    // Fetch tags for all documents in the page in a single query, and organize it by tag ID.
    let mut tags_by_document: HashMap<i64, Vec<i64>> = HashMap::new();
    if !document_ids.is_empty() {
        let tag_rows: Vec<(i64, i64)> = tag_documents::table
            .filter(tag_documents::document_id.eq_any(&document_ids))
            .select((tag_documents::document_id, tag_documents::tag_id))
            .load::<(i64, i64)>(&mut db)
            .await
            .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to list tags for documents"))?;

        for (document_id, tag_id) in tag_rows {
            tags_by_document
                .entry(document_id)
                .or_insert_with(Vec::new)
                .push(tag_id);
        }
    }

    // Construct the final list of document views, attaching metadata to each document.
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

    Ok(Json(ResourceList {
        total,
        page,
        per_page,
        items,
    }))
}

pub async fn list_index_values(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Path(id): Path<i64>,
) -> Result<Json<Vec<DocumentIndexValue>>, ApiError> {
    let items = document_index_documents::table
        .inner_join(document_index_values::table)
        .filter(document_index_documents::document_id.eq(id))
        .select(DocumentIndexValue::as_select())
        .order(document_index_values::id.asc())
        .load::<DocumentIndexValue>(&mut db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to list document_index_values"))?;

    Ok(Json(items))
}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list).post(create))
        // Allow up to 100MB uploads for document creation.
        .layer(DefaultBodyLimit::max(100 * 1024 * 1024 * 1024))
        .route("/{id}", get(get_by_id).patch(update).delete(delete))
        .route("/{id}/classify-document", post(classify_document))
        .route("/{id}/test-classifier-block", post(test_classifier_block))
        .route("/{id}/test-template", post(test_template))
        .route("/{id}/index-values", get(list_index_values))
        .route("/{id}/thumbnail", get(thumbnail_get))
        .route("/{id}/generate-thumbnail", post(generate_thumbnail))
        .route("/{id}/process-file-pages", post(process_file_pages))
}
