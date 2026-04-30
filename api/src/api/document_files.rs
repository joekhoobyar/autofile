use std::sync::Arc;

use crate::application::document_files::{
    BufferedDocumentFileUpload, buffer_document_file_field, cleanup_buffered_document_file_upload,
    delete_uploaded_document_file_from_s3, insert_document_file, upload_document_file_to_s3,
};
use crate::application::jobs::{FastJob, MediumJob};
use crate::domain::document_files::DocumentFileView;
use crate::schema::{document_files, documents};
use crate::shared::app_state::AppState;
use crate::shared::auth::{AuthUser, sign_download, verify_download};
use crate::shared::extractors::DbConn;
use crate::shared::s3::serve_s3_file;
use crate::shared::util::{ApiError, diesel_to_http};

use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Multipart, Path, Query, State},
    http::{HeaderMap, StatusCode, header},
    response::Response,
    routing::{get, post},
};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;

use apalis::prelude::*;

const DOWNLOAD_TTL_SECONDS: i64 = 120;

#[derive(serde::Serialize)]
pub struct DownloadTicketResponse {
    pub url: String,
    pub expires_in: i64,
}

#[derive(serde::Deserialize)]
pub struct DownloadQuery {
    pub t: Option<String>,
}

async fn parse_create_multipart(
    multipart: &mut Multipart,
) -> Result<BufferedDocumentFileUpload, ApiError> {
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

        if field_name.as_str() == "file" {
            if let Some(upload) = &file_temp {
                cleanup_buffered_document_file_upload(upload).await;
                return Err(ApiError::bad_request("Only one file upload is supported"));
            }
            file_temp = Some(buffer_document_file_field(&mut field).await?);
        }
    }

    file_temp.ok_or_else(|| ApiError::bad_request("Missing required field: file"))
}

pub async fn list(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Path(document_id): Path<i64>,
) -> Result<Json<Vec<DocumentFileView>>, ApiError> {
    let rows = document_files::table
        .filter(document_files::document_id.eq(document_id))
        .select(DocumentFileView::as_select())
        .order(document_files::id.asc())
        .load::<DocumentFileView>(&mut db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to list document_files"))?;

    Ok(Json(rows))
}

pub async fn get_by_ids(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Path((document_id, id)): Path<(i64, i64)>,
) -> Result<Json<DocumentFileView>, ApiError> {
    let row = document_files::table
        .filter(document_files::document_id.eq(document_id))
        .filter(document_files::id.eq(id))
        .select(DocumentFileView::as_select())
        .first::<DocumentFileView>(&mut db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to fetch document_file"))?;

    Ok(Json(row))
}

pub async fn create(
    user: AuthUser,
    State(state): State<Arc<AppState>>,
    DbConn(mut db): DbConn,
    Path(document_id): Path<i64>,
    mut multipart: Multipart,
) -> Result<Json<DocumentFileView>, ApiError> {
    let file_temp = parse_create_multipart(&mut multipart).await?;
    let file_info = upload_document_file_to_s3(&state, file_temp).await?;
    let file_info_for_cleanup = file_info.clone();

    let fast_jobs = state.fast_jobs.as_ref().clone();
    let medium_jobs = state.medium_jobs.as_ref().clone();
    let thumbnail_enqueue_failed = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let thumbnail_enqueue_failed_for_tx = Arc::clone(&thumbnail_enqueue_failed);
    let pages_enqueue_failed = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let pages_enqueue_failed_for_tx = Arc::clone(&pages_enqueue_failed);

    let result = db
        .build_transaction()
        .run::<_, diesel::result::Error, _>(|conn| {
            Box::pin(async move {
                let mut fast_jobs = fast_jobs;
                let mut medium_jobs = medium_jobs;

                documents::table
                    .find(document_id)
                    .select(documents::id)
                    .first::<i64>(conn)
                    .await?;

                let existing_file_count = document_files::table
                    .filter(document_files::document_id.eq(document_id))
                    .count()
                    .get_result::<i64>(conn)
                    .await?;

                let inserted_file =
                    insert_document_file(conn, document_id, file_info, user.user_id).await?;

                if let Err(_) = medium_jobs
                    .push(MediumJob::ProcessFilePages {
                        document_file_id: inserted_file.id,
                    })
                    .await
                {
                    pages_enqueue_failed_for_tx.store(true, std::sync::atomic::Ordering::Relaxed);
                    return Err(diesel::result::Error::RollbackTransaction);
                }

                if existing_file_count == 0 {
                    if let Err(_) = fast_jobs
                        .push(FastJob::GenerateThumbnail {
                            document_file_id: inserted_file.id,
                            page: 1,
                            width: 800,
                        })
                        .await
                    {
                        thumbnail_enqueue_failed_for_tx
                            .store(true, std::sync::atomic::Ordering::Relaxed);
                        return Err(diesel::result::Error::RollbackTransaction);
                    }
                }

                Ok(DocumentFileView {
                    id: inserted_file.id,
                    document_id: inserted_file.document_id,
                    filename: inserted_file.filename,
                    content_type: inserted_file.content_type,
                    size: inserted_file.size,
                    pages: inserted_file.pages,
                    created_at: inserted_file.created_at,
                    created_by: inserted_file.created_by,
                    updated_at: inserted_file.updated_at,
                    updated_by: inserted_file.updated_by,
                })
            })
        })
        .await;

    match result {
        Ok(document_file) => Ok(Json(document_file)),
        Err(e) => {
            delete_uploaded_document_file_from_s3(&state, &file_info_for_cleanup).await;
            if matches!(e, diesel::result::Error::RollbackTransaction) {
                let pages_failed = pages_enqueue_failed
                    .as_ref()
                    .load(std::sync::atomic::Ordering::Relaxed);
                let thumbnail_failed = thumbnail_enqueue_failed
                    .as_ref()
                    .load(std::sync::atomic::Ordering::Relaxed);
                if pages_failed {
                    Err(ApiError::new(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Failed to enqueue file pages job",
                    ))
                } else if thumbnail_failed {
                    Err(ApiError::new(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Failed to enqueue thumbnail job",
                    ))
                } else {
                    Err(ApiError::new(
                        diesel_to_http(e),
                        "Failed to create document_file",
                    ))
                }
            } else {
                Err(ApiError::new(
                    diesel_to_http(e),
                    "Failed to create document_file",
                ))
            }
        }
    }
}

pub async fn thumbnail_get(
    _user: AuthUser,
    State(state): State<Arc<AppState>>,
    DbConn(mut db): DbConn,
    Path((document_id, id)): Path<(i64, i64)>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let (s3_prefix, updated_at) = document_files::table
        .filter(document_files::document_id.eq(document_id))
        .filter(document_files::id.eq(id))
        .select((document_files::s3_prefix, document_files::updated_at))
        .first::<(String, chrono::DateTime<chrono::Utc>)>(&mut db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to fetch document file thumbnail"))?;

    let s3_key = format!("{}/_thumb.png", s3_prefix);
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

pub async fn download(
    State(state): State<Arc<AppState>>,
    DbConn(mut db): DbConn,
    Path((document_id, id)): Path<(i64, i64)>,
    query: Query<DownloadQuery>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    if let Some(token) = &query.t {
        let claims = verify_download(&state.jwt_secret, token)
            .map_err(|_| ApiError::unauthorized("Invalid or expired download ticket"))?;
        if claims.document_id != document_id || claims.document_file_id != id {
            return Err(ApiError::unauthorized("Invalid download ticket"));
        }
    } else {
        let auth_header = headers
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .ok_or_else(|| ApiError::unauthorized("Missing Authorization header"))?;
        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or_else(|| ApiError::unauthorized("Invalid Authorization header format"))?;
        if token.is_empty() {
            return Err(ApiError::unauthorized("Empty token"));
        }
        crate::shared::auth::verify_access(&state.jwt_secret, token)
            .map_err(|_| ApiError::unauthorized("Invalid or expired token"))?;
    }

    let (s3_prefix, filename, content_type, updated_at) = document_files::table
        .filter(document_files::document_id.eq(document_id))
        .filter(document_files::id.eq(id))
        .select((
            document_files::s3_prefix,
            document_files::filename,
            document_files::content_type,
            document_files::updated_at,
        ))
        .first::<(
            String,
            String,
            Option<String>,
            chrono::DateTime<chrono::Utc>,
        )>(&mut db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to fetch document file download"))?;

    let s3_key = format!("{}/{}", s3_prefix, filename);
    let mut response = serve_s3_file(
        state.as_ref(),
        &headers,
        &s3_key,
        Some(updated_at),
        "File not available",
        content_type.as_deref(),
    )
    .await?;

    if let Ok(value) =
        header::HeaderValue::from_str(&format!("attachment; filename={:?}", filename))
    {
        response
            .headers_mut()
            .insert(header::CONTENT_DISPOSITION, value);
    }

    Ok(response)
}

pub async fn create_download_ticket(
    user: AuthUser,
    State(state): State<Arc<AppState>>,
    DbConn(mut db): DbConn,
    Path((document_id, id)): Path<(i64, i64)>,
) -> Result<Json<DownloadTicketResponse>, ApiError> {
    document_files::table
        .filter(document_files::document_id.eq(document_id))
        .filter(document_files::id.eq(id))
        .select(document_files::id)
        .first::<i64>(&mut db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to fetch document file"))?;

    let token = sign_download(
        &state.jwt_secret,
        user.user_id,
        document_id,
        id,
        DOWNLOAD_TTL_SECONDS,
    )
    .map_err(|_| ApiError::internal_server_error("Token error"))?;

    Ok(Json(DownloadTicketResponse {
        url: format!("api/v1/documents/{document_id}/files/{id}/download?t={token}"),
        expires_in: DOWNLOAD_TTL_SECONDS,
    }))
}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .layer(DefaultBodyLimit::max(100 * 1024 * 1024))
        .route("/{document_id}/files", get(list).post(create))
        .route("/{document_id}/files/{id}", get(get_by_ids))
        .route("/{document_id}/files/{id}/thumbnail", get(thumbnail_get))
        .route(
            "/{document_id}/files/{id}/download-ticket",
            post(create_download_ticket),
        )
        .route("/{document_id}/files/{id}/download", get(download))
}
