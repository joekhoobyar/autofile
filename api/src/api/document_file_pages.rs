use std::sync::Arc;

use crate::application::document_files::{
    get_document_file_page_image_key, list_document_file_ocr_pages, list_document_file_pages,
};
use crate::domain::document_files::{DocumentFileOcrPage, DocumentFilePage};
use crate::shared::app_state::AppState;
use crate::shared::auth::AuthUser;
use crate::shared::errors::ApiError;
use crate::shared::extractors::DbConn;
use crate::shared::s3::serve_s3_file;

use axum::{
    Json,
    extract::{Path, State},
    http::HeaderMap,
    response::Response,
};
use utoipa_axum::{router::OpenApiRouter, routes};

#[utoipa::path(
    get,
    path = "/{document_id}/files/{document_file_id}/pages",
    tag = "document-files",
    security(("bearer" = [])),
    params(
        ("document_id" = i64, Path, description = "Document ID"),
        ("document_file_id" = i64, Path, description = "File ID"),
    ),
    responses(
        (status = 200, description = "Extracted text pages in page order", body = Vec<DocumentFilePage>),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 404, description = "File not found", body = ApiError),
    )
)]
pub async fn list(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Path((document_id, document_file_id)): Path<(i64, i64)>,
) -> Result<Json<Vec<DocumentFilePage>>, ApiError> {
    let rows = list_document_file_pages(&mut db, document_id, document_file_id).await?;
    Ok(Json(rows))
}

#[utoipa::path(
    get,
    path = "/{document_id}/files/{document_file_id}/ocr-pages",
    tag = "document-files",
    security(("bearer" = [])),
    params(
        ("document_id" = i64, Path, description = "Document ID"),
        ("document_file_id" = i64, Path, description = "File ID"),
    ),
    responses(
        (status = 200, description = "OCR text pages in page order", body = Vec<DocumentFileOcrPage>),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 404, description = "File not found", body = ApiError),
    )
)]
pub async fn list_ocr(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Path((document_id, document_file_id)): Path<(i64, i64)>,
) -> Result<Json<Vec<DocumentFileOcrPage>>, ApiError> {
    let rows = list_document_file_ocr_pages(&mut db, document_id, document_file_id).await?;
    Ok(Json(rows))
}

/**
 * Streams a rendered page image directly from S3.
 */
#[utoipa::path(
    get,
    path = "/{document_id}/files/{document_file_id}/pages/{page_number}/image",
    tag = "document-files",
    security(("bearer" = [])),
    params(
        ("document_id" = i64, Path, description = "Document ID"),
        ("document_file_id" = i64, Path, description = "File ID"),
        ("page_number" = i32, Path, description = "1-based page number"),
    ),
    responses(
        (status = 200, description = "Rendered page image bytes", content_type = "image/png", body = Vec<u8>),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 404, description = "Page not available", body = ApiError),
    )
)]
pub async fn page_image_get(
    _user: AuthUser,
    State(state): State<Arc<AppState>>,
    DbConn(mut db): DbConn,
    Path((document_id, document_file_id, page_number)): Path<(i64, i64, i32)>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let s3_key =
        get_document_file_page_image_key(&mut db, document_id, document_file_id, page_number)
            .await?;
    serve_s3_file(
        state.as_ref(),
        &headers,
        &s3_key,
        None,
        "Page not available",
        Some("image/png"),
    )
    .await
}

pub fn routes() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(list))
        .routes(routes!(list_ocr))
        .routes(routes!(page_image_get))
}
