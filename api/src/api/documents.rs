use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::AppState;
use crate::schema::{documents, document_files};
use crate::domain::documents::Document;
use crate::domain::document_files::DocumentFile;
use crate::infrastructure::s3::{delete_from_s3, upload_to_s3};
use crate::infrastructure::thumbnail::GenerateThumbnail;
use crate::shared::auth::AuthUser;
use crate::shared::extractors::DbConn;
use crate::shared::util::{diesel_to_http, ApiError};

use aws_sdk_s3::primitives::ByteStream;
use serde::Deserialize;

use axum::{
    Router,
    routing::get,
    Json,
    http::StatusCode,
    extract::{Path, Query, Multipart, State},
};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use chrono::Utc;
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

#[derive(Debug, Deserialize)]
pub struct ListDocumentsQuery {
    // 1-based page number
    pub page: Option<i64>,
    // items per page (cap it)
    pub per_page: Option<i64>,
    // optional substring search
    pub q: Option<String>,
    document_type_id: Option<i64>,
}

pub async fn get_by_id(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Path(id): Path<i64>,
) -> Result<Json<Document>, ApiError> {
    let row = documents::table
        .find(id)
        .select(Document::as_select())
        .first::<Document>(&mut db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to fetch document"))?;

    Ok(Json(row))
}

async fn create(
    user: AuthUser,
    State(state): State<Arc<AppState>>,
    DbConn(mut db): DbConn,
    mut multipart: Multipart,
) -> Result<Json<Document>, ApiError> {
    // Parse multipart form fields
    let mut title: Option<String> = None;
    let mut document_type_id: Option<i64> = None;
    let mut file_data: Option<(String, Vec<u8>, Option<String>)> = None;

    while let Some(field) = multipart.next_field().await
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
                let mut filename = field.file_name()
                    .ok_or_else(|| ApiError::bad_request("File field missing filename"))?
                    .to_string();
                let content_type = field.content_type().map(|ct| ct.to_string());
                let data = field.bytes().await
                    .map_err(|e| ApiError::bad_request(&format!("Failed to read file data: {}", e)))?;

                // Normalize filename to prevent clashes with existing thumbnails
                if filename == "_thumb.png" {
                    filename = "thumb.png".to_string();
                }

                file_data = Some((filename, data.to_vec(), content_type));
            }
            _ => {
                // Ignore unknown fields
            }
        }
    }

    // Validate required fields
    let title = title.ok_or_else(|| ApiError::bad_request("Missing required field: title"))?;
    let document_type_id = document_type_id
        .ok_or_else(|| ApiError::bad_request("Missing required field: document_type_id"))?;

    // Handle optional file upload
    let file_info = if let Some((filename, data, content_type)) = file_data {
        let file_size = data.len() as i64;

        // Generate UUID for s3_prefix
        let s3_prefix = Uuid::new_v4().to_string();
        let s3_key = format!("{}/{}", s3_prefix, filename);

        // Upload to S3
        let body = ByteStream::from(data.to_vec());
        upload_to_s3(
            &state.s3_client,
            &state.s3_bucket,
            &s3_key,
            body,
            content_type.as_deref(),
        )
        .await
        .map_err(|e| ApiError::internal_server_error(&format!("S3 upload failed: {}", e)))?;

        Some((s3_prefix, filename, content_type, file_size))
    } else {
        None
    };

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
) -> Result<Json<Vec<Document>>, ApiError> {
    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(50).clamp(1, 200);
    let offset = (page - 1) * per_page;

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

    let rows = query
        .order(documents::id.desc())
        .limit(per_page)
        .offset(offset)
        .select(Document::as_select())
        .load::<Document>(&mut db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to list documents"))?;

    Ok(Json(rows))
}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/{id}", get(get_by_id).patch(update))
}
