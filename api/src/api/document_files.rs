use std::sync::Arc;

use crate::shared::app_state::AppState;
use crate::domain::document_files::DocumentFileView;
use crate::schema::document_files;
use crate::shared::auth::{AuthUser, sign_download, verify_download};
use crate::shared::extractors::DbConn;
use crate::shared::s3::serve_s3_file;
use crate::shared::util::{ApiError, diesel_to_http};

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::{HeaderMap, header},
    response::Response,
    routing::{get, post},
};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;

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
        .route("/{document_id}/files", get(list))
        .route("/{document_id}/files/{id}", get(get_by_ids))
        .route("/{document_id}/files/{id}/thumbnail", get(thumbnail_get))
        .route(
            "/{document_id}/files/{id}/download-ticket",
            post(create_download_ticket),
        )
        .route("/{document_id}/files/{id}/download", get(download))
}
