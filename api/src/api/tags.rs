use std::sync::Arc;

use crate::application::tags::{
    ListTagsQuery, NewTag, TagChangeset, create_tag, delete_tag, get_tag, get_tag_by_slug,
    list_tags, update_tag,
};
use crate::domain::tags::{Tag, TagView};
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
    path = "/{id}",
    tag = "tags",
    security(("bearer" = [])),
    params(("id" = i64, Path, description = "Tag ID")),
    responses(
        (status = 200, description = "Tag", body = Tag),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 404, description = "Tag not found", body = ApiError),
    )
)]
pub async fn get_by_id(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Path(id): Path<i64>,
) -> Result<Json<Tag>, ApiError> {
    let row = get_tag(&mut db, id).await?;
    Ok(Json(row))
}

#[utoipa::path(
    get,
    path = "/by-slug/{slug}",
    tag = "tags",
    security(("bearer" = [])),
    params(("slug" = String, Path, description = "Exact tag slug")),
    responses(
        (status = 200, description = "Tag", body = Tag),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 404, description = "Tag not found", body = ApiError),
    )
)]
pub async fn get_by_slug(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Path(slug): Path<String>,
) -> Result<Json<Tag>, ApiError> {
    let row = get_tag_by_slug(&mut db, slug).await?;
    Ok(Json(row))
}

#[utoipa::path(
    post,
    path = "/",
    tag = "tags",
    security(("bearer" = [])),
    request_body = NewTag,
    responses(
        (status = 200, description = "Created tag", body = Tag),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 409, description = "Slug is already taken", body = ApiError),
        (status = 422, description = "Invalid slug", body = ApiError),
    )
)]
async fn create(
    user: AuthUser,
    DbConn(mut db): DbConn,
    Json(input): Json<NewTag>,
) -> Result<Json<Tag>, ApiError> {
    let inserted = create_tag(&mut db, user.user_id, input).await?;
    Ok(Json(inserted))
}

#[utoipa::path(
    patch,
    path = "/{id}",
    tag = "tags",
    security(("bearer" = [])),
    params(("id" = i64, Path, description = "Tag ID")),
    request_body = TagChangeset,
    responses(
        (status = 200, description = "Updated tag", body = Tag),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 404, description = "Tag not found", body = ApiError),
    )
)]
async fn update(
    user: AuthUser,
    DbConn(mut db): DbConn,
    Path(id): Path<i64>,
    Json(input): Json<TagChangeset>,
) -> Result<Json<Tag>, ApiError> {
    let updated = update_tag(&mut db, user.user_id, id, input).await?;
    Ok(Json(updated))
}

#[utoipa::path(
    delete,
    path = "/{id}",
    tag = "tags",
    security(("bearer" = [])),
    params(("id" = i64, Path, description = "Tag ID")),
    responses(
        (status = 200, description = "Tag deleted"),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 404, description = "Tag not found", body = ApiError),
    )
)]
async fn delete(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Path(id): Path<i64>,
) -> Result<Json<()>, ApiError> {
    delete_tag(&mut db, id).await?;
    Ok(Json(()))
}

#[utoipa::path(
    get,
    path = "/",
    tag = "tags",
    security(("bearer" = [])),
    params(ListTagsQuery),
    responses(
        (status = 200, description = "Paginated tag list with document counts", body = ResourceList<TagView>),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
    )
)]
pub async fn list(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Query(params): Query<ListTagsQuery>,
) -> Result<Json<ResourceList<TagView>>, ApiError> {
    Ok(Json(list_tags(&mut db, params).await?))
}

pub fn routes() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(list))
        .routes(routes!(create))
        .routes(routes!(get_by_id))
        .routes(routes!(update))
        .routes(routes!(delete))
        .routes(routes!(get_by_slug))
}
