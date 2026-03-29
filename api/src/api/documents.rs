use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::AppState;
use crate::application::documents::get_document_view;
use crate::schema::{cabinet_documents, document_files, document_index_documents, document_metadatas, documents, metadata_types, tag_documents};
use crate::domain::documents::{Document, DocumentView};
use crate::domain::document_files::DocumentFile;
use crate::infrastructure::s3::{delete_from_s3, delete_prefix_from_s3, upload_to_s3};
use crate::application::document_thumbnails::GenerateThumbnail;
use crate::shared::auth::AuthUser;
use crate::shared::extractors::DbConn;
use crate::shared::util::{diesel_to_http, write_field_to_temp_file, ApiError, ResourceList};

use aws_sdk_s3::primitives::ByteStream;
use diesel::dsl::exists;
use serde::Deserialize;
use tokio_util::io::ReaderStream;

use axum::{
    Router,
    routing::get,
    Json,
    http::{StatusCode, header, HeaderMap},
    response::Response,
    body::Body,
    extract::{Path, Query, Multipart, State},
};
use diesel::prelude::*;
use diesel_async::{AsyncConnection, RunQueryDsl};
use chrono::Utc;
use httpdate::{fmt_http_date, parse_http_date};
use uuid::Uuid;

use apalis::prelude::*;

#[derive(Debug, Deserialize, Insertable)]
#[diesel(table_name = documents)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct NewDocument {
    title: String,
    document_type_id: i64,
}

#[derive(Debug, Deserialize, AsChangeset)]
#[diesel(table_name = documents)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct DocumentChangeset {
    title: Option<String>,
    document_type_id: Option<i64>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = document_files)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct NewDocumentFile {
    document_id: i64,
    s3_prefix: String,
    filename: String,
    content_type: Option<String>,
    size: i64,
    created_by: i64,
    updated_by: i64,
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
    // optional substring search
    pub q: Option<String>,
    // optional document type search
    pub document_type_id: Option<i64>,
    // optional cabinet search
    pub cabinet_id: Option<i64>,
    // optional tag search
    pub tag_id: Option<i64>,
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
    file_temp: Option<(std::path::PathBuf, String, Option<String>, i64)>,
}

async fn parse_create_multipart(
    multipart: &mut Multipart,
) -> Result<ParsedMultipart, ApiError> {
    let mut title: Option<String> = None;
    let mut document_type_id: Option<i64> = None;
    let mut file_temp: Option<(std::path::PathBuf, String, Option<String>, i64)> = None;

    while let Some(mut field) = multipart.next_field().await
        .map_err(|e| ApiError::bad_request(&format!("Failed to read multipart field: {}", e)))? {

        let field_name = field.name()
            .ok_or_else(|| ApiError::bad_request("Field missing name"))?
            .to_string();

        match field_name.as_str() {
            "title" => {
                let value = field.text().await
                    .map_err(|e| ApiError::bad_request(&format!("Failed to read title: {}", e)))?;
                title = Some(value);
            }
            "document_type_id" => {
                let value = field.text().await
                    .map_err(|e| ApiError::bad_request(&format!("Failed to read document_type_id: {}", e)))?;
                document_type_id = Some(value.parse::<i64>()
                    .map_err(|_| ApiError::bad_request("Invalid document_type_id"))?);
            }
            "file" => {
                if file_temp.is_some() {
                    return Err(ApiError::bad_request("Only one file upload is supported"));
                }
                let mut filename = field.file_name()
                    .ok_or_else(|| ApiError::bad_request("File field missing filename"))?
                    .to_string();
                let content_type = field.content_type().map(|ct| ct.to_string());

                if filename == "_thumb.png" {
                    filename = "thumb.png".to_string();
                }

                let temp_upload = write_field_to_temp_file(&mut field).await?;
                file_temp = Some((temp_upload.path, filename, content_type, temp_upload.size));
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

async fn upload_temp_file_to_s3(
    state: &AppState,
    temp_path: std::path::PathBuf,
    filename: String,
    content_type: Option<String>,
    size: i64,
) -> Result<(String, String, Option<String>, i64), ApiError> {
    let s3_prefix = Uuid::new_v4().to_string();
    let s3_key = format!("{}/{}", s3_prefix, filename);
    let upload_body = match ByteStream::from_path(&temp_path).await {
        Ok(body) => body,
        Err(e) => {
            let _ = tokio::fs::remove_file(&temp_path).await;
            return Err(ApiError::internal_server_error(&format!("Failed to read temp file: {}", e)));
        }
    };
    let upload_result = upload_to_s3(
        &state.s3_client,
        &state.s3_bucket,
        &s3_key,
        upload_body,
        content_type.as_deref(),
    )
    .await;
    let _ = tokio::fs::remove_file(&temp_path).await;
    upload_result
        .map_err(|e| ApiError::internal_server_error(&format!("S3 upload failed: {}", e)))?;

    Ok((s3_prefix, filename, content_type, size))
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
    let prefixes = db.transaction::<_, diesel::result::Error, _>(move |conn| {
        Box::pin(async move {
            // Delete the cabinet document associations
            diesel::delete(cabinet_documents::table.filter(cabinet_documents::document_id.eq(id)))
                .execute(conn)
                .await?;

            // Delete the document metadata associations
            diesel::delete(document_metadatas::table.filter(document_metadatas::document_id.eq(id)))
                .execute(conn)
                .await?;

            // Delete the document files, fetching the S3 keys for later deletion.
            let prefixes: Vec<String> = diesel::delete(document_files::table.filter(document_files::document_id.eq(id)))
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
    // Find the document by ID and get the S3 key for the thumbnail, plus the document's updated_at timestamp for caching purposes.
    let (s3_thumbnail, updated_at) = documents::table
        .find(id)
        .select((documents::s3_thumbnail, documents::updated_at))
        .first::<(Option<String>, chrono::DateTime<chrono::Utc>)>(&mut db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to fetch document thumbnail"))?;

    let s3_key = s3_thumbnail
        .ok_or_else(|| ApiError::not_found("Thumbnail not available"))?;

    // Support conditional GET with If-Modified-Since header to avoid unnecessary S3 requests and data transfer.
    if let Some(if_modified_since) = headers
        .get(header::IF_MODIFIED_SINCE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| parse_http_date(value).ok())
    {
        let updated_at_system: std::time::SystemTime = updated_at.into();
        if updated_at_system <= if_modified_since {
            let mut response = Response::new(Body::empty());
            *response.status_mut() = StatusCode::NOT_MODIFIED;
            let headers = response.headers_mut();
            let last_modified = fmt_http_date(updated_at_system);
            if let Ok(value) = header::HeaderValue::from_str(&last_modified) {
                headers.insert(header::LAST_MODIFIED, value);
            }
            headers.insert(
                header::CACHE_CONTROL,
                header::HeaderValue::from_static("public, must-revalidate"),
            );
            return Ok(response);
        }
    }

    // Prepare an HTTP response to stream the thumbnail directly from S3.
    let object = state
        .s3_client
        .get_object()
        .bucket(state.s3_bucket.as_str())
        .key(&s3_key)
        .send()
        .await
        .map_err(|e| ApiError::internal_server_error(&format!("S3 download failed: {}", e)))?;
    let body = Body::from_stream(ReaderStream::new(object.body.into_async_read()));
    let mut response = Response::new(body);

    // Prepare the HTTP headers, including content type, caching, and last modified.
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("image/png"),
    );
    let last_modified = fmt_http_date(updated_at.into());
    if let Ok(value) = header::HeaderValue::from_str(&last_modified) {
        headers.insert(header::LAST_MODIFIED, value);
    }
    headers.insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("public, must-revalidate"),
    );

    Ok(response)
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

    let mut file_info: Option<(String, String, Option<String>, i64)> = None;

    // Validate required fields
    let title = match title {
        Some(value) => value,
        None => {
            if let Some((temp_path, _, _, _)) = &file_temp {
                let _ = tokio::fs::remove_file(temp_path).await;
            }
            return Err(ApiError::bad_request("Missing required field: title"));
        }
    };
    let document_type_id = match document_type_id {
        Some(value) => value,
        None => {
            if let Some((temp_path, _, _, _)) = &file_temp {
                let _ = tokio::fs::remove_file(temp_path).await;
            }
            return Err(ApiError::bad_request("Missing required field: document_type_id"));
        }
    };

    // Upload the temp file to S3, then delete it.
    if let Some((temp_path, filename, content_type, file_size)) = file_temp.take() {
        file_info = Some(upload_temp_file_to_s3(
            &state,
            temp_path,
            filename,
            content_type,
            file_size,
        ).await?);
    }

    // Clone file_info for potential cleanup in error path
    let file_info_for_cleanup = file_info.clone();

    // Clone the thumbnail job queue handle for use inside the transaction closure
    let thumb_jobs = state.thumb_jobs.as_ref().clone();
    let thumb_enqueue_failed = Arc::new(AtomicBool::new(false));
    let thumb_enqueue_failed_for_tx = Arc::clone(&thumb_enqueue_failed);

    // Begin database transaction
    let result = db.build_transaction()
        .run::<_, diesel::result::Error, _>(|conn| {
            Box::pin(async move {
                let mut thumb_jobs = thumb_jobs;
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
                if let Some((s3_prefix, filename, content_type, file_size)) = file_info {
                    let inserted_file = diesel::insert_into(document_files::table)
                        .values(&NewDocumentFile {
                            document_id: inserted_document.id,
                            s3_prefix,
                            filename,
                            content_type,
                            size: file_size,
                            created_by: user.user_id,
                            updated_by: user.user_id,
                        })
                        .returning(DocumentFile::as_returning())
                        .get_result(conn)
                        .await?;

                    if let Err(_) = thumb_jobs
                        .push(GenerateThumbnail {
                            document_file_id: inserted_file.id,
                            page: 1,
                            width: 800,
                        })
                        .await
                    {
                        thumb_enqueue_failed_for_tx.store(true, Ordering::Relaxed);
                        return Err(diesel::result::Error::RollbackTransaction);
                    }
                }

                Ok(inserted_document)
            })
        })
        .await;

    match result {
        Ok(document) => Ok(Json(document)),
        Err(e) => {
            // On transaction failure, attempt S3 cleanup (best-effort)
            if let Some((s3_prefix, filename, _, _)) = file_info_for_cleanup {
                let s3_key = format!("{}/{}", s3_prefix, filename);
                let _ = delete_from_s3(
                    &state.s3_client,
                    &state.s3_bucket,
                    &s3_key,
                ).await;
            }
            if matches!(e, diesel::result::Error::RollbackTransaction)
                && thumb_enqueue_failed.as_ref().load(Ordering::Relaxed)
            {
                Err(ApiError::new(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to enqueue thumbnail job",
                ))
            } else {
                Err(ApiError::new(diesel_to_http(e), "Failed to create document"))
            }
        }
    }
}

async fn update(
    user: AuthUser,
    DbConn(mut db): DbConn,
    Path(id): Path<i64>,
    Json(input): Json<DocumentChangeset>,
) -> Result<Json<Document>, ApiError> {
    // Update + return the updated row in one round-trip.
    let updated: Document =
        diesel::update(documents::table.filter(documents::id.eq(id)))
            .set((
                &input,
                documents::updated_by.eq(user.user_id),
                documents::updated_at.eq(Utc::now()),
            ))
            .returning(Document::as_returning())
            .get_result(&mut db)
            .await
            .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to update document"))?;

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

    let base_filter = || -> documents::BoxedQuery<'_, diesel::pg::Pg> {
        // Start with a boxed query so we can conditionally add filters.
        let mut query = documents::table.into_boxed();

        // Optional search: case-insensitive substring on title
        if let Some(q) = params.q.as_deref().filter(|s| !s.is_empty()) {
            let pattern = format!("%{}%", q);
            query = query.filter(
                documents::title.ilike(pattern)
            );
        }

        // Filter by document type
        if let Some(id) = params.document_type_id {
            query = query.filter(documents::document_type_id.eq(id));
        }

        // Filter by cabinet ID
        if let Some(id) = params.cabinet_id {
            let subquery = cabinet_documents::table
                .filter(cabinet_documents::cabinet_id.eq(id))
                .filter(cabinet_documents::document_id.eq(documents::id));

            query = query.filter(exists(subquery));
        }

        // Filter by tag ID
        if let Some(id) = params.tag_id {
            let subquery = tag_documents::table
                .filter(tag_documents::tag_id.eq(id))
                .filter(tag_documents::document_id.eq(documents::id));

            query = query.filter(exists(subquery));
        }

        // Filter by document index value ID
        if let Some(id) = params.document_index_value_id {
            let subquery = document_index_documents::table
                .filter(document_index_documents::document_index_value_id.eq(id))
                .filter(document_index_documents::document_id.eq(documents::id));

            query = query.filter(exists(subquery));
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
        (Some(DocumentSortField::Title), Some(true)) =>
            query.order((documents::title.desc(), documents::id.asc())), // tie-breaker
        (Some(DocumentSortField::Title), _) =>
            query.order((documents::title.asc(), documents::id.asc())), // tie-breaker
        (Some(DocumentSortField::CreatedAt), Some(true)) =>
            query.order((documents::created_at.desc(), documents::id.asc())), // tie-breaker
        (Some(DocumentSortField::CreatedAt), _) =>
            query.order((documents::created_at.asc(), documents::id.asc())), // tie-breaker
        (Some(DocumentSortField::UpdatedAt), Some(true)) =>
            query.order((documents::updated_at.desc(), documents::id.asc())), // tie-breaker
        (Some(DocumentSortField::UpdatedAt), _) =>
            query.order((documents::updated_at.asc(), documents::id.asc())), // tie-breaker

        (Some(DocumentSortField::Id), Some(true)) =>
            query.order(documents::id.desc()),
        _ =>
            query.order(documents::id.asc()),
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
            .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to list cabinets for documents"))?;

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
            .select((
                tag_documents::document_id,
                tag_documents::tag_id,
            ))
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
            metadata: metadata_by_document.remove(&doc.id).unwrap_or_default(),
            cabinet_ids: cabinets_by_document.remove(&doc.id).unwrap_or_default(),
            tag_ids: tags_by_document.remove(&doc.id).unwrap_or_default(),
            created_by: doc.created_by,
            created_at: doc.created_at,
            updated_by: doc.updated_by,
            updated_at: doc.updated_at,
        })
        .collect();

    Ok(Json(ResourceList { total, page, per_page, items }))
}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/{id}", get(get_by_id).patch(update).delete(delete))
        .route("/{id}/thumbnail", get(thumbnail_get))
}
