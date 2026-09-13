use std::sync::Arc;

use crate::application::tag_documents::{
    ListTagDocumentsQuery, NewTagDocument, delete_tag_document, delete_tag_documents,
    get_tag_document, list_tag_documents, upsert_tag_documents,
};
use crate::domain::tag_documents::TagDocument;
use crate::shared::app_state::AppState;
use crate::shared::auth::AuthUser;
use crate::shared::errors::ApiError;
use crate::shared::extractors::DbConn;
use crate::shared::responses::ResourceList;

use axum::extract::State;

use axum::{
    Json,
    extract::{Path, Query},
};
use utoipa_axum::{router::OpenApiRouter, routes};

#[utoipa::path(
    get,
    path = "/{tag_id}/documents/{document_id}",
    tag = "tags",
    security(("bearer" = [])),
    params(
        ("tag_id" = i64, Path, description = "Tag ID"),
        ("document_id" = i64, Path, description = "Document ID"),
    ),
    responses(
        (status = 200, description = "Tag-document association", body = TagDocument),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 404, description = "Association not found", body = ApiError),
    )
)]
pub async fn get_by_ids(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Path((tag_id, document_id)): Path<(i64, i64)>,
) -> Result<Json<TagDocument>, ApiError> {
    let row = get_tag_document(&mut db, tag_id, document_id).await?;

    Ok(Json(row))
}

#[utoipa::path(
    post,
    path = "/{tag_id}/documents",
    tag = "tags",
    security(("bearer" = [])),
    params(("tag_id" = i64, Path, description = "Tag ID")),
    request_body = Vec<NewTagDocument>,
    responses(
        (status = 200, description = "Created or updated associations", body = Vec<TagDocument>),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 422, description = "Unprocessable request", body = ApiError),
    )
)]
async fn upsert(
    user: AuthUser,
    State(state): State<Arc<AppState>>,
    DbConn(mut db): DbConn,
    Path(tag_id): Path<i64>,
    Json(input): Json<Vec<NewTagDocument>>,
) -> Result<Json<Vec<TagDocument>>, ApiError> {
    let items = upsert_tag_documents(state, &mut db, user.user_id, tag_id, input).await?;

    Ok(Json(items))
}

#[utoipa::path(
    get,
    path = "/{tag_id}/documents",
    tag = "tags",
    security(("bearer" = [])),
    params(
        ("tag_id" = i64, Path, description = "Tag ID"),
        ListTagDocumentsQuery,
    ),
    responses(
        (status = 200, description = "Paginated tag-document associations", body = ResourceList<TagDocument>),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
    )
)]
pub async fn list(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Query(params): Query<ListTagDocumentsQuery>,
) -> Result<Json<ResourceList<TagDocument>>, ApiError> {
    let list = list_tag_documents(&mut db, params).await?;

    Ok(Json(list))
}

#[utoipa::path(
    post,
    path = "/{tag_id}/documents/delete",
    tag = "tags",
    security(("bearer" = [])),
    params(("tag_id" = i64, Path, description = "Tag ID")),
    request_body = Vec<i64>,
    responses(
        (status = 200, description = "Associations removed"),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
    )
)]
async fn delete(
    _user: AuthUser,
    State(state): State<Arc<AppState>>,
    DbConn(mut db): DbConn,
    Path(tag_id): Path<i64>,
    Json(input): Json<Vec<i64>>,
) -> Result<Json<()>, ApiError> {
    delete_tag_documents(state, &mut db, tag_id, input).await?;

    Ok(Json(()))
}

#[utoipa::path(
    delete,
    path = "/{tag_id}/documents/{document_id}",
    tag = "tags",
    security(("bearer" = [])),
    params(
        ("tag_id" = i64, Path, description = "Tag ID"),
        ("document_id" = i64, Path, description = "Document ID"),
    ),
    responses(
        (status = 200, description = "Association removed"),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 404, description = "Association not found", body = ApiError),
    )
)]
async fn delete_junction(
    _user: AuthUser,
    State(state): State<Arc<AppState>>,
    DbConn(mut db): DbConn,
    Path((tag_id, document_id)): Path<(i64, i64)>,
) -> Result<Json<()>, ApiError> {
    delete_tag_document(state, &mut db, tag_id, document_id).await?;

    Ok(Json(()))
}

pub fn routes() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(list))
        .routes(routes!(upsert))
        .routes(routes!(delete))
        .routes(routes!(get_by_ids))
        .routes(routes!(delete_junction))
}
