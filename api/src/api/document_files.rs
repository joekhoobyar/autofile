use std::sync::Arc;

use crate::AppState;
use crate::domain::document_files::DocumentFileView;
use crate::schema::document_files;
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

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/{document_id}/files", get(list))
}
