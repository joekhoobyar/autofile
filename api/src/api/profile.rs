use std::sync::Arc;

use crate::api::auth::{AccessTokenResponse, issue_tokens};
use crate::application::users::{
    ChangePasswordInput, UpdateProfileInput, change_password, get_profile, update_profile,
};
use crate::domain::users::User;
use crate::shared::app_state::AppState;
use crate::shared::auth::{AuthUser, PasswordChangeUser};
use crate::shared::errors::ApiError;
use crate::shared::extractors::DbConn;

use axum::{Json, extract::State};
use tower_cookies::Cookies;
use utoipa_axum::{router::OpenApiRouter, routes};

#[utoipa::path(
    get,
    path = "/",
    tag = "profile",
    security(("bearer" = [])),
    responses(
        (status = 200, description = "Current user's profile", body = User),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
    )
)]
async fn get_current(
    AuthUser { user_id }: AuthUser,
    DbConn(mut db): DbConn,
) -> Result<Json<User>, ApiError> {
    Ok(Json(get_profile(&mut db, user_id).await?))
}

#[utoipa::path(
    patch,
    path = "/",
    tag = "profile",
    security(("bearer" = [])),
    request_body = UpdateProfileInput,
    responses(
        (status = 200, description = "Updated profile", body = User),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 409, description = "Email address is already registered", body = ApiError),
    )
)]
async fn update_current(
    AuthUser { user_id }: AuthUser,
    DbConn(mut db): DbConn,
    Json(input): Json<UpdateProfileInput>,
) -> Result<Json<User>, ApiError> {
    Ok(Json(update_profile(&mut db, user_id, input).await?))
}

#[utoipa::path(
    post,
    path = "/password",
    tag = "profile",
    security(("bearer" = [])),
    request_body = ChangePasswordInput,
    responses(
        (status = 200, description = "Password changed; returns a fresh access token and refreshes the session cookie", body = AccessTokenResponse),
        (status = 400, description = "Invalid request, e.g. password shorter than 12 characters", body = ApiError),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
    )
)]
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

pub fn routes() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(get_current))
        .routes(routes!(update_current))
        .routes(routes!(update_password))
}
