use std::sync::Arc;

use crate::application::users::{
    ListUsersInput, UpdateUserInput, delete_user, get_user_by_id, get_user_by_username, list_users,
    update_user,
};
use crate::domain::users::User;
use crate::shared::app_state::AppState;
use crate::shared::auth::AuthUser;
use crate::shared::extractors::DbConn;
use crate::shared::util::{ApiError, ResourceList};

use axum::{
    Json, Router,
    extract::{Path, Query},
    routing::get,
};

pub async fn get_by_id(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Path(id): Path<i64>,
) -> Result<Json<User>, ApiError> {
    Ok(Json(get_user_by_id(&mut db, id).await?))
}

pub async fn get_by_username(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Path(username): Path<String>,
) -> Result<Json<User>, ApiError> {
    Ok(Json(get_user_by_username(&mut db, username).await?))
}

async fn update(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Path(id): Path<i64>,
    Json(input): Json<UpdateUserInput>,
) -> Result<Json<User>, ApiError> {
    Ok(Json(update_user(&mut db, id, input).await?))
}

async fn delete(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Path(id): Path<i64>,
) -> Result<Json<()>, ApiError> {
    delete_user(&mut db, id).await?;
    Ok(Json(()))
}

pub async fn list(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Query(params): Query<ListUsersInput>,
) -> Result<Json<ResourceList<User>>, ApiError> {
    Ok(Json(list_users(&mut db, params).await?))
}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list))
        .route("/{id}", get(get_by_id).patch(update).delete(delete))
        .route("/by-username/{username}", get(get_by_username))
}
