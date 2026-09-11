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
    Json, Router,
    extract::{Path, Query},
    routing::{get, post},
};

pub async fn get_by_id(
    _admin: AdminUser,
    DbConn(mut db): DbConn,
    Path(id): Path<i64>,
) -> Result<Json<User>, ApiError> {
    Ok(Json(get_user_by_id(&mut db, id).await?))
}

pub async fn get_by_username(
    _admin: AdminUser,
    DbConn(mut db): DbConn,
    Path(username): Path<String>,
) -> Result<Json<User>, ApiError> {
    Ok(Json(get_user_by_username(&mut db, username).await?))
}

async fn update(
    AdminUser { user_id }: AdminUser,
    DbConn(mut db): DbConn,
    Path(id): Path<i64>,
    Json(input): Json<UpdateUserInput>,
) -> Result<Json<User>, ApiError> {
    Ok(Json(update_user(&mut db, user_id, id, input).await?))
}

async fn delete(
    AdminUser { user_id }: AdminUser,
    DbConn(mut db): DbConn,
    Path(id): Path<i64>,
) -> Result<Json<()>, ApiError> {
    delete_user(&mut db, user_id, id).await?;
    Ok(Json(()))
}

async fn restore(
    _admin: AdminUser,
    DbConn(mut db): DbConn,
    Path(id): Path<i64>,
) -> Result<Json<User>, ApiError> {
    Ok(Json(restore_user(&mut db, id).await?))
}

pub async fn list(
    _admin: AdminUser,
    DbConn(mut db): DbConn,
    Query(params): Query<ListUsersInput>,
) -> Result<Json<ResourceList<User>>, ApiError> {
    Ok(Json(list_users(&mut db, params).await?))
}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list))
        .route("/{id}", get(get_by_id).patch(update).delete(delete))
        .route("/{id}/restore", post(restore))
        .route("/by-username/{username}", get(get_by_username))
}
