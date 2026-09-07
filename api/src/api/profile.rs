use std::sync::Arc;

use crate::api::auth::{AccessTokenResponse, issue_tokens};
use crate::application::users::{
    ChangePasswordInput, UpdateProfileInput, change_password, get_profile, update_profile,
};
use crate::domain::users::User;
use crate::shared::app_state::AppState;
use crate::shared::auth::{AuthUser, PasswordChangeUser};
use crate::shared::extractors::DbConn;
use crate::shared::util::ApiError;

use axum::{Json, Router, extract::State, routing::get};
use tower_cookies::Cookies;

async fn get_current(
    AuthUser { user_id }: AuthUser,
    DbConn(mut db): DbConn,
) -> Result<Json<User>, ApiError> {
    Ok(Json(get_profile(&mut db, user_id).await?))
}

async fn update_current(
    AuthUser { user_id }: AuthUser,
    DbConn(mut db): DbConn,
    Json(input): Json<UpdateProfileInput>,
) -> Result<Json<User>, ApiError> {
    Ok(Json(update_profile(&mut db, user_id, input).await?))
}

async fn update_password(
    State(state): State<Arc<AppState>>,
    cookies: Cookies,
    PasswordChangeUser { user_id }: PasswordChangeUser,
    DbConn(mut db): DbConn,
    Json(input): Json<ChangePasswordInput>,
) -> Result<Json<AccessTokenResponse>, ApiError> {
    let user = change_password(&mut db, user_id, input).await?;
    Ok(Json(issue_tokens(&state, &cookies, &user)?))
}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(get_current).patch(update_current))
        .route("/password", axum::routing::post(update_password))
}
