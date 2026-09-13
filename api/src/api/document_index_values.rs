use std::sync::Arc;

use crate::application::document_index_values::{
    ListDocumentIndexValuesQuery, list_document_index_value_ancestors, list_document_index_values,
};
use crate::domain::document_indexes::{DocumentIndexValue, DocumentIndexValueView};
use crate::shared::app_state::AppState;
use crate::shared::auth::AuthUser;
use crate::shared::errors::ApiError;
use crate::shared::extractors::DbConn;
use crate::shared::responses::ResourceList;

use axum::{
    Json,
    extract::{Path, Query},
};
use utoipa_axum::{router::OpenApiRouter, routes};

#[utoipa::path(
    get,
    path = "/{document_index_id}/values",
    tag = "document-indexes",
    security(("bearer" = [])),
    params(
        ("document_index_id" = i64, Path, description = "Document Index ID"),
        ListDocumentIndexValuesQuery,
    ),
    responses(
        (status = 200, description = "Paginated index value list with document counts", body = ResourceList<DocumentIndexValueView>),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
    )
)]
pub async fn list(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Path(document_index_id): Path<i64>,
    Query(params): Query<ListDocumentIndexValuesQuery>,
) -> Result<Json<ResourceList<DocumentIndexValueView>>, ApiError> {
    Ok(Json(
        list_document_index_values(&mut db, document_index_id, params).await?,
    ))
}

#[utoipa::path(
    get,
    path = "/{document_index_id}/values/{id}/ancestors",
    tag = "document-indexes",
    security(("bearer" = [])),
    params(
        ("document_index_id" = i64, Path, description = "Document Index ID"),
        ("id" = i64, Path, description = "Index Value ID"),
    ),
    responses(
        (status = 200, description = "Ancestor values from the root down to the parent", body = Vec<DocumentIndexValue>),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 404, description = "Index value not found", body = ApiError),
    )
)]
pub async fn ancestors(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Path((document_index_id, id)): Path<(i64, i64)>,
) -> Result<Json<Vec<DocumentIndexValue>>, ApiError> {
    let items = list_document_index_value_ancestors(&mut db, document_index_id, id).await?;
    Ok(Json(items))
}

pub fn routes() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(list))
        .routes(routes!(ancestors))
}
