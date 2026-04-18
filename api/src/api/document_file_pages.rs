use std::sync::Arc;

use crate::domain::document_files::{DocumentFileOcrPage, DocumentFilePage};
use crate::schema::{document_file_ocr_pages, document_file_pages, document_files};
use crate::shared::app_state::AppState;
use crate::shared::auth::AuthUser;
use crate::shared::extractors::DbConn;
use crate::shared::s3::serve_s3_file;
use crate::shared::util::{ApiError, diesel_to_http};

use axum::{
    Json, Router,
    extract::{Path, State},
    http::HeaderMap,
    response::Response,
    routing::get,
};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;

pub async fn list(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Path((document_id, document_file_id)): Path<(i64, i64)>,
) -> Result<Json<Vec<DocumentFilePage>>, ApiError> {
    let rows = document_file_pages::table
        .inner_join(
            document_files::table.on(document_files::id.eq(document_file_pages::document_file_id)),
        )
        .filter(document_files::document_id.eq(document_id))
        .filter(document_file_pages::document_file_id.eq(document_file_id))
        .select(DocumentFilePage::as_select())
        .order(document_file_pages::page_number.asc())
        .load::<DocumentFilePage>(&mut db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to list document_file_pages"))?;

    Ok(Json(rows))
}

pub async fn list_ocr(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Path((document_id, document_file_id)): Path<(i64, i64)>,
) -> Result<Json<Vec<DocumentFileOcrPage>>, ApiError> {
    let rows = document_file_ocr_pages::table
        .inner_join(
            document_files::table
                .on(document_files::id.eq(document_file_ocr_pages::document_file_id)),
        )
        .filter(document_files::document_id.eq(document_id))
        .filter(document_file_ocr_pages::document_file_id.eq(document_file_id))
        .select(DocumentFileOcrPage::as_select())
        .order(document_file_ocr_pages::page_number.asc())
        .load::<DocumentFileOcrPage>(&mut db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to list document_file_ocr_pages"))?;

    Ok(Json(rows))
}

/**
 * Streams a rendered page image directly from S3.
 */
pub async fn page_image_get(
    _user: AuthUser,
    State(state): State<Arc<AppState>>,
    DbConn(mut db): DbConn,
    Path((document_id, document_file_id, page_number)): Path<(i64, i64, i32)>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let s3_prefix = document_files::table
        .filter(document_files::document_id.eq(document_id))
        .filter(document_files::id.eq(document_file_id))
        .select(document_files::s3_prefix)
        .first::<String>(&mut db)
        .await
        .map_err(|e| {
            if matches!(e, diesel::result::Error::NotFound) {
                ApiError::not_found("Document file not found")
            } else {
                ApiError::new(diesel_to_http(e), "Failed to fetch document file")
            }
        })?;

    let s3_key = format!("{}/pages/{}.png", s3_prefix, page_number);
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

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/{document_id}/files/{document_file_id}/pages", get(list))
        .route(
            "/{document_id}/files/{document_file_id}/ocr-pages",
            get(list_ocr),
        )
        .route(
            "/{document_id}/files/{document_file_id}/pages/{page_number}/image",
            get(page_image_get),
        )
}
