use std::sync::Arc;

use crate::AppState;
use crate::domain::document_files::DocumentFileView;
use crate::schema::document_files;
use crate::shared::auth::AuthUser;
use crate::shared::extractors::DbConn;
use crate::shared::s3::serve_s3_file;
use crate::shared::util::{ApiError, diesel_to_http};

use axum::{
    Json, Router,
    extract::{Path, State},
    http::{HeaderMap, header},
    response::Response,
    routing::get,
};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;

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
    _user: AuthUser,
    State(state): State<Arc<AppState>>,
    DbConn(mut db): DbConn,
    Path((document_id, id)): Path<(i64, i64)>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
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

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/{document_id}/files", get(list))
        .route("/{document_id}/files/{id}", get(get_by_ids))
        .route("/{document_id}/files/{id}/thumbnail", get(thumbnail_get))
        .route("/{document_id}/files/{id}/download", get(download))
}
