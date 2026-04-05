use std::sync::Arc;

use crate::AppState;
use crate::domain::document_files::DocumentFilePage;
use crate::schema::{document_file_pages, document_files};
use crate::shared::auth::AuthUser;
use crate::shared::extractors::DbConn;
use crate::shared::util::{ApiError, diesel_to_http};

use aws_sdk_s3::error::SdkError;
use axum::{
    Json, Router,
    body::Body,
    extract::{Path, State},
    http::header,
    response::Response,
    routing::get,
};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use httpdate::fmt_http_date;
use tokio_util::io::ReaderStream;

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

/**
 * Streams a rendered page image directly from S3.
 */
pub async fn page_image_get(
    _user: AuthUser,
    State(state): State<Arc<AppState>>,
    DbConn(mut db): DbConn,
    Path((document_id, document_file_id, page_number)): Path<(i64, i64, i32)>,
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
    let object = state
        .s3_client
        .get_object()
        .bucket(state.s3_bucket.as_str())
        .key(&s3_key)
        .send()
        .await
        .map_err(|e| match e {
            SdkError::ServiceError(service_error) if service_error.err().is_no_such_key() => {
                ApiError::not_found("Page not available")
            }
            _ => ApiError::internal_server_error(&format!("S3 download failed: {e}")),
        })?;

    let last_modified = object.last_modified().copied();
    let content_length = object.content_length();
    let body = Body::from_stream(ReaderStream::new(object.body.into_async_read()));
    let mut response = Response::new(body);
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("image/png"),
    );
    headers.insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("public, must-revalidate"),
    );
    if let Some(last_modified) = last_modified {
        if let Ok(system_time) = std::time::SystemTime::try_from(last_modified) {
            let last_modified = fmt_http_date(system_time);
            if let Ok(value) = header::HeaderValue::from_str(&last_modified) {
                headers.insert(header::LAST_MODIFIED, value);
            }
        }
    }
    if let Some(content_length) = content_length {
        if content_length > 0 {
            if let Ok(value) = header::HeaderValue::from_str(&content_length.to_string()) {
                headers.insert(header::CONTENT_LENGTH, value);
            }
        }
    }

    Ok(response)
}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/{document_id}/files/{document_file_id}/pages", get(list))
        .route(
            "/{document_id}/files/{document_file_id}/pages/{page_number}/image",
            get(page_image_get),
        )
}
