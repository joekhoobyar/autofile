use std::sync::Arc;

use crate::application::users::{
    ListUsersInput, UpdateUserInput, delete_user, get_user_by_id, get_user_by_username, list_users,
    restore_user, update_user,
};
use crate::domain::users::User;
use crate::shared::app_state::AppState;
use crate::shared::auth::AdminUser;
use crate::shared::extractors::DbConn;
use crate::shared::util::{ApiError, ResourceList};

use axum::{
    Json,
    extract::{Path, Query},
};
use utoipa_axum::{router::OpenApiRouter, routes};

#[utoipa::path(
    get,
    path = "/{id}",
    tag = "users",
    security(("bearer" = [])),
    params(("id" = i64, Path, description = "User ID")),
    responses(
        (status = 200, description = "User", body = User),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Admin role required", body = ApiError),
        (status = 404, description = "User not found", body = ApiError),
    )
)]
pub async fn get_by_id(
    _admin: AdminUser,
    DbConn(mut db): DbConn,
    Path(id): Path<i64>,
) -> Result<Json<User>, ApiError> {
    Ok(Json(get_user_by_id(&mut db, id).await?))
}

#[utoipa::path(
    get,
    path = "/by-username/{username}",
    tag = "users",
    security(("bearer" = [])),
    params(("username" = String, Path, description = "Exact username")),
    responses(
        (status = 200, description = "User", body = User),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Admin role required", body = ApiError),
        (status = 404, description = "User not found", body = ApiError),
    )
)]
pub async fn get_by_username(
    _admin: AdminUser,
    DbConn(mut db): DbConn,
    Path(username): Path<String>,
) -> Result<Json<User>, ApiError> {
    Ok(Json(get_user_by_username(&mut db, username).await?))
}

#[utoipa::path(
    patch,
    path = "/{id}",
    tag = "users",
    security(("bearer" = [])),
    params(("id" = i64, Path, description = "User ID")),
    request_body = UpdateUserInput,
    responses(
        (status = 200, description = "Updated user", body = User),
        (status = 400, description = "Invalid request, e.g. updating the system user or your own role", body = ApiError),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Admin role required", body = ApiError),
        (status = 404, description = "User not found", body = ApiError),
    )
)]
async fn update(
    AdminUser { user_id }: AdminUser,
    DbConn(mut db): DbConn,
    Path(id): Path<i64>,
    Json(input): Json<UpdateUserInput>,
) -> Result<Json<User>, ApiError> {
    Ok(Json(update_user(&mut db, user_id, id, input).await?))
}

#[utoipa::path(
    delete,
    path = "/{id}",
    tag = "users",
    security(("bearer" = [])),
    params(("id" = i64, Path, description = "User ID")),
    responses(
        (status = 200, description = "User soft-deleted and disabled"),
        (status = 400, description = "Invalid request, e.g. deleting the system user or your own account", body = ApiError),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Admin role required", body = ApiError),
        (status = 404, description = "User not found", body = ApiError),
    )
)]
async fn delete(
    AdminUser { user_id }: AdminUser,
    DbConn(mut db): DbConn,
    Path(id): Path<i64>,
) -> Result<Json<()>, ApiError> {
    delete_user(&mut db, user_id, id).await?;
    Ok(Json(()))
}

#[utoipa::path(
    post,
    path = "/{id}/restore",
    tag = "users",
    security(("bearer" = [])),
    params(("id" = i64, Path, description = "User ID")),
    responses(
        (status = 200, description = "Restored user (disabled)", body = User),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Admin role required", body = ApiError),
        (status = 404, description = "User not found", body = ApiError),
    )
)]
async fn restore(
    _admin: AdminUser,
    DbConn(mut db): DbConn,
    Path(id): Path<i64>,
) -> Result<Json<User>, ApiError> {
    Ok(Json(restore_user(&mut db, id).await?))
}

#[utoipa::path(
    get,
    path = "/",
    tag = "users",
    security(("bearer" = [])),
    params(ListUsersInput),
    responses(
        (status = 200, description = "Paginated user list", body = ResourceList<User>),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Admin role required", body = ApiError),
    )
)]
pub async fn list(
    _admin: AdminUser,
    DbConn(mut db): DbConn,
    Query(params): Query<ListUsersInput>,
) -> Result<Json<ResourceList<User>>, ApiError> {
    Ok(Json(list_users(&mut db, params).await?))
}

pub fn routes() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(list))
        .routes(routes!(get_by_id))
        .routes(routes!(update))
        .routes(routes!(delete))
        .routes(routes!(restore))
        .routes(routes!(get_by_username))
}
