use std::sync::Arc;

use crate::application::document_files::{
    BufferedDocumentFileUpload, buffer_document_file_field, cleanup_buffered_document_file_upload,
    create_document_file, delete_document_file, ensure_document_file_available, get_document_file,
    get_document_file_download_metadata, get_document_file_thumbnail_metadata, list_document_files,
    rescan_document_file,
};
use crate::domain::document_files::DocumentFileView;
use crate::shared::app_state::AppState;
use crate::shared::auth::{AuthUser, sign_download, verify_download};
use crate::shared::config::MAX_UPLOAD_MULTIPART_OVERHEAD_BYTES;
use crate::shared::errors::ApiError;
use crate::shared::extractors::DbConn;
use crate::shared::s3::serve_s3_file;

use axum::{
    Json,
    extract::{DefaultBodyLimit, Multipart, Path, Query, State},
    http::{HeaderMap, header},
    response::Response,
};
use utoipa_axum::{router::OpenApiRouter, routes};

const DOWNLOAD_TTL_SECONDS: i64 = 120;

#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct DownloadTicketResponse {
    pub url: String,
    pub expires_in: i64,
}

#[derive(serde::Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct DownloadQuery {
    /// Short-lived download ticket from the download-ticket endpoint.
    /// When omitted, a bearer access token is required instead.
    pub t: Option<String>,
}

/// Documented shape of the `multipart/form-data` body accepted by file upload.
/// This type is never constructed; it only describes the wire format because
/// the handler parses the multipart stream manually.
#[derive(utoipa::ToSchema)]
#[allow(dead_code)]
struct UploadDocumentFileMultipart {
    /// Uploaded file bytes (required).
    #[schema(value_type = String, format = Binary)]
    file: String,
    /// Optional per-upload virus scan selection. Ignored when virus scanning is disabled.
    virus_scan: Option<bool>,
}

struct ParsedCreateMultipart {
    file_temp: BufferedDocumentFileUpload,
    virus_scan: Option<bool>,
}

async fn parse_create_multipart(
    multipart: &mut Multipart,
    max_bytes: u64,
) -> Result<ParsedCreateMultipart, ApiError> {
    let mut file_temp: Option<BufferedDocumentFileUpload> = None;
    let mut virus_scan: Option<bool> = None;

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
            "file" => {
                if let Some(upload) = &file_temp {
                    cleanup_buffered_document_file_upload(upload).await;
                    return Err(ApiError::bad_request("Only one file upload is supported"));
                }
                file_temp = Some(buffer_document_file_field(&mut field, max_bytes).await?);
            }
            "virus_scan" | "scan_for_viruses" => {
                let value = field.text().await.map_err(|e| {
                    ApiError::bad_request(&format!("Failed to read virus_scan: {}", e))
                })?;
                virus_scan = Some(parse_multipart_bool(&value, "virus_scan")?);
            }
            _ => {}
        }
    }

    Ok(ParsedCreateMultipart {
        file_temp: file_temp
            .ok_or_else(|| ApiError::bad_request("Missing required field: file"))?,
        virus_scan,
    })
}

fn parse_multipart_bool(value: &str, field_name: &str) -> Result<bool, ApiError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Ok(true),
        "false" | "0" | "no" | "off" => Ok(false),
        _ => Err(ApiError::bad_request(&format!("Invalid {field_name}"))),
    }
}

#[utoipa::path(
    get,
    path = "/{document_id}/files",
    tag = "document-files",
    security(("bearer" = [])),
    params(("document_id" = i64, Path, description = "Document ID")),
    responses(
        (status = 200, description = "File list for the document", body = Vec<DocumentFileView>),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 404, description = "Document not found", body = ApiError),
    )
)]
pub async fn list(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Path(document_id): Path<i64>,
) -> Result<Json<Vec<DocumentFileView>>, ApiError> {
    let rows = list_document_files(&mut db, document_id).await?;
    Ok(Json(rows))
}

#[utoipa::path(
    get,
    path = "/{document_id}/files/{id}",
    tag = "document-files",
    security(("bearer" = [])),
    params(
        ("document_id" = i64, Path, description = "Document ID"),
        ("id" = i64, Path, description = "File ID"),
    ),
    responses(
        (status = 200, description = "File metadata", body = DocumentFileView),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 404, description = "File not found", body = ApiError),
    )
)]
pub async fn get_by_ids(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Path((document_id, id)): Path<(i64, i64)>,
) -> Result<Json<DocumentFileView>, ApiError> {
    let row = get_document_file(&mut db, document_id, id).await?;
    Ok(Json(row))
}

#[utoipa::path(
    post,
    path = "/{document_id}/files",
    tag = "document-files",
    security(("bearer" = [])),
    params(("document_id" = i64, Path, description = "Document ID")),
    request_body(
        content = UploadDocumentFileMultipart,
        content_type = "multipart/form-data",
        description = "Single uploaded file"
    ),
    responses(
        (status = 200, description = "Uploaded file metadata", body = DocumentFileView),
        (status = 400, description = "Invalid upload, e.g. missing file", body = ApiError),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 404, description = "Document not found", body = ApiError),
        (status = 500, description = "Storage or job enqueue failure", body = ApiError),
    )
)]
pub async fn create(
    user: AuthUser,
    State(state): State<Arc<AppState>>,
    DbConn(mut db): DbConn,
    Path(document_id): Path<i64>,
    mut multipart: Multipart,
) -> Result<Json<DocumentFileView>, ApiError> {
    let ParsedCreateMultipart {
        file_temp,
        virus_scan,
    } = parse_create_multipart(&mut multipart, state.max_upload_bytes as u64).await?;
    let document_file = create_document_file(
        state,
        &mut db,
        user.user_id,
        document_id,
        file_temp,
        virus_scan,
    )
    .await?;
    Ok(Json(document_file))
}

#[utoipa::path(
    delete,
    path = "/{document_id}/files/{id}",
    tag = "document-files",
    security(("bearer" = [])),
    params(
        ("document_id" = i64, Path, description = "Document ID"),
        ("id" = i64, Path, description = "File ID"),
    ),
    responses(
        (status = 200, description = "File and its pages deleted"),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 404, description = "File not found", body = ApiError),
        (status = 409, description = "Cannot delete the last file in a document", body = ApiError),
        (status = 500, description = "Storage failure", body = ApiError),
    )
)]
pub async fn delete(
    _user: AuthUser,
    State(state): State<Arc<AppState>>,
    DbConn(mut db): DbConn,
    Path((document_id, id)): Path<(i64, i64)>,
) -> Result<Json<()>, ApiError> {
    delete_document_file(state, &mut db, document_id, id).await?;
    Ok(Json(()))
}

#[utoipa::path(
    post,
    path = "/{document_id}/files/{id}/rescan",
    tag = "document-files",
    security(("bearer" = [])),
    params(
        ("document_id" = i64, Path, description = "Document ID"),
        ("id" = i64, Path, description = "File ID"),
    ),
    responses(
        (status = 200, description = "File submitted for virus rescan", body = DocumentFileView),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 404, description = "File not found", body = ApiError),
        (status = 409, description = "Virus scanning disabled or file already scanning", body = ApiError),
        (status = 500, description = "Failed to enqueue virus scan job", body = ApiError),
    )
)]
pub async fn rescan(
    user: AuthUser,
    State(state): State<Arc<AppState>>,
    DbConn(mut db): DbConn,
    Path((document_id, id)): Path<(i64, i64)>,
) -> Result<Json<DocumentFileView>, ApiError> {
    Ok(Json(
        rescan_document_file(state, &mut db, user.user_id, document_id, id).await?,
    ))
}

#[utoipa::path(
    get,
    path = "/{document_id}/files/{id}/thumbnail",
    tag = "document-files",
    security(("bearer" = [])),
    params(
        ("document_id" = i64, Path, description = "Document ID"),
        ("id" = i64, Path, description = "File ID"),
    ),
    responses(
        (status = 200, description = "PNG thumbnail bytes", content_type = "image/png", body = Vec<u8>),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 404, description = "Thumbnail not available", body = ApiError),
    )
)]
pub async fn thumbnail_get(
    _user: AuthUser,
    State(state): State<Arc<AppState>>,
    DbConn(mut db): DbConn,
    Path((document_id, id)): Path<(i64, i64)>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let (s3_key, updated_at) =
        get_document_file_thumbnail_metadata(&mut db, document_id, id).await?;
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

#[utoipa::path(
    get,
    path = "/{document_id}/files/{id}/download",
    tag = "document-files",
    security(("bearer" = [])),
    params(
        ("document_id" = i64, Path, description = "Document ID"),
        ("id" = i64, Path, description = "File ID"),
        DownloadQuery,
    ),
    responses(
        (status = 200, description = "File bytes as an attachment; alternatively authorized with a download ticket query parameter", content_type = "application/octet-stream", body = Vec<u8>),
        (status = 401, description = "Missing or invalid ticket/token", body = ApiError),
        (status = 404, description = "File not available", body = ApiError),
    )
)]
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

    let file = get_document_file_download_metadata(&mut db, document_id, id).await?;
    let mut response = serve_s3_file(
        state.as_ref(),
        &headers,
        &file.s3_key,
        Some(file.updated_at),
        "File not available",
        file.content_type.as_deref(),
    )
    .await?;

    if let Ok(value) =
        header::HeaderValue::from_str(&format!("attachment; filename={:?}", file.filename))
    {
        response
            .headers_mut()
            .insert(header::CONTENT_DISPOSITION, value);
    }

    Ok(response)
}

#[utoipa::path(
    post,
    path = "/{document_id}/files/{id}/download-ticket",
    tag = "document-files",
    security(("bearer" = [])),
    params(
        ("document_id" = i64, Path, description = "Document ID"),
        ("id" = i64, Path, description = "File ID"),
    ),
    responses(
        (status = 200, description = "Short-lived download URL usable without an Authorization header", body = DownloadTicketResponse),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 404, description = "File not found", body = ApiError),
        (status = 500, description = "Token error", body = ApiError),
    )
)]
pub async fn create_download_ticket(
    user: AuthUser,
    State(state): State<Arc<AppState>>,
    DbConn(mut db): DbConn,
    Path((document_id, id)): Path<(i64, i64)>,
) -> Result<Json<DownloadTicketResponse>, ApiError> {
    ensure_document_file_available(&mut db, document_id, id).await?;

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

pub fn routes(max_upload_bytes: usize) -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .layer(DefaultBodyLimit::max(
            max_upload_bytes.saturating_add(MAX_UPLOAD_MULTIPART_OVERHEAD_BYTES),
        ))
        .routes(routes!(list))
        .routes(routes!(create))
        .routes(routes!(get_by_ids))
        .routes(routes!(delete))
        .routes(routes!(rescan))
        .routes(routes!(thumbnail_get))
        .routes(routes!(create_download_ticket))
        .routes(routes!(download))
}
