use std::sync::Arc;

use crate::application::document_indexes::{
    DocumentIndexChangeset, ListDocumentIndexesQuery, NewDocumentIndex, create_document_index,
    delete_document_index, get_document_index, get_document_index_by_slug, list_document_indexes,
    rebuild_document_index, update_document_index,
};
use crate::domain::document_indexes::{DocumentIndex, DocumentIndexView};
use crate::shared::app_state::AppState;
use crate::shared::auth::AuthUser;
use crate::shared::errors::ApiError;
use crate::shared::extractors::DbConn;
use crate::shared::responses::ResourceList;

use axum::{
    Json,
    extract::{Path, Query, State},
};
use utoipa_axum::{router::OpenApiRouter, routes};

#[utoipa::path(
    get,
    path = "/{id}",
    tag = "document-indexes",
    security(("bearer" = [])),
    params(("id" = i64, Path, description = "Document Index ID")),
    responses(
        (status = 200, description = "Document Index", body = DocumentIndex),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 404, description = "Document Index not found", body = ApiError),
    )
)]
pub async fn get_by_id(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Path(id): Path<i64>,
) -> Result<Json<DocumentIndex>, ApiError> {
    let row = get_document_index(&mut db, id).await?;

    Ok(Json(row))
}

#[utoipa::path(
    get,
    path = "/by-slug/{slug}",
    tag = "document-indexes",
    security(("bearer" = [])),
    params(("slug" = String, Path, description = "Exact document index slug")),
    responses(
        (status = 200, description = "Document Index", body = DocumentIndex),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 404, description = "Document Index not found", body = ApiError),
    )
)]
pub async fn get_by_slug(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Path(slug): Path<String>,
) -> Result<Json<DocumentIndex>, ApiError> {
    let row = get_document_index_by_slug(&mut db, slug).await?;

    Ok(Json(row))
}

#[utoipa::path(
    post,
    path = "/",
    tag = "document-indexes",
    security(("bearer" = [])),
    request_body = NewDocumentIndex,
    responses(
        (status = 200, description = "Created Document Index", body = DocumentIndex),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 409, description = "Slug is already taken", body = ApiError),
        (status = 422, description = "Invalid slug", body = ApiError),
    )
)]
async fn create(
    user: AuthUser,
    DbConn(mut db): DbConn,
    Json(input): Json<NewDocumentIndex>,
) -> Result<Json<DocumentIndex>, ApiError> {
    let inserted = create_document_index(&mut db, user.user_id, input).await?;

    Ok(Json(inserted))
}

#[utoipa::path(
    patch,
    path = "/{id}",
    tag = "document-indexes",
    security(("bearer" = [])),
    params(("id" = i64, Path, description = "Document Index ID")),
    request_body = DocumentIndexChangeset,
    responses(
        (status = 200, description = "Updated Document Index", body = DocumentIndex),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 404, description = "Document Index not found", body = ApiError),
    )
)]
async fn update(
    user: AuthUser,
    DbConn(mut db): DbConn,
    Path(id): Path<i64>,
    Json(input): Json<DocumentIndexChangeset>,
) -> Result<Json<DocumentIndex>, ApiError> {
    let updated = update_document_index(&mut db, user.user_id, id, input).await?;

    Ok(Json(updated))
}

#[utoipa::path(
    delete,
    path = "/{id}",
    tag = "document-indexes",
    security(("bearer" = [])),
    params(("id" = i64, Path, description = "Document Index ID")),
    responses(
        (status = 200, description = "Document Index and its templates, values, and assignments deleted"),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 404, description = "Document Index not found", body = ApiError),
    )
)]
async fn delete(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Path(id): Path<i64>,
) -> Result<Json<()>, ApiError> {
    delete_document_index(&mut db, id).await?;

    Ok(Json(()))
}

#[utoipa::path(
    post,
    path = "/{id}/rebuild",
    tag = "document-indexes",
    security(("bearer" = [])),
    params(("id" = i64, Path, description = "Document Index ID")),
    responses(
        (status = 200, description = "Rebuild job enqueued"),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 404, description = "Document Index not found", body = ApiError),
        (status = 500, description = "Failed to enqueue rebuild job", body = ApiError),
    )
)]
async fn rebuild(
    _user: AuthUser,
    State(state): State<Arc<AppState>>,
    DbConn(mut db): DbConn,
    Path(id): Path<i64>,
) -> Result<Json<()>, ApiError> {
    rebuild_document_index(state, &mut db, id).await?;

    Ok(Json(()))
}

#[utoipa::path(
    get,
    path = "/",
    tag = "document-indexes",
    security(("bearer" = [])),
    params(ListDocumentIndexesQuery),
    responses(
        (status = 200, description = "Paginated Document Index list with document counts", body = ResourceList<DocumentIndexView>),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
    )
)]
pub async fn list(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Query(params): Query<ListDocumentIndexesQuery>,
) -> Result<Json<ResourceList<DocumentIndexView>>, ApiError> {
    Ok(Json(list_document_indexes(&mut db, params).await?))
}

pub fn routes() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(list))
        .routes(routes!(create))
        .routes(routes!(get_by_id))
        .routes(routes!(update))
        .routes(routes!(delete))
        .routes(routes!(rebuild))
        .routes(routes!(get_by_slug))
}
