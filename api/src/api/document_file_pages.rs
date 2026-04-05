use std::sync::Arc;

use crate::AppState;
use crate::domain::document_files::DocumentFilePage;
use crate::schema::{document_file_pages, document_files};
use crate::shared::auth::AuthUser;
use crate::shared::extractors::DbConn;
use crate::shared::util::{diesel_to_http, ApiError};

use axum::{
    Router,
    routing::get,
    Json,
    extract::Path,
};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;

pub async fn list(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Path((document_id, document_file_id)): Path<(i64, i64)>,
) -> Result<Json<Vec<DocumentFilePage>>, ApiError> {
    let rows = document_file_pages::table
        .inner_join(document_files::table.on(
            document_files::id.eq(document_file_pages::document_file_id),
        ))
        .filter(document_files::document_id.eq(document_id))
        .filter(document_file_pages::document_file_id.eq(document_file_id))
        .select(DocumentFilePage::as_select())
        .order(document_file_pages::page_number.asc())
        .load::<DocumentFilePage>(&mut db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to list document_file_pages"))?;

    Ok(Json(rows))
}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/{document_id}/files/{document_file_id}/pages", get(list))
}
