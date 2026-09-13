use std::sync::Arc;

use crate::application::auth::{
    find_user_for_login, find_user_for_refresh, register_user, validate_login_user,
    validate_refresh_user,
};
use crate::domain::users::User;
use crate::is_production;
use crate::shared::app_state::AppState;
use crate::shared::auth::{sign_access, sign_refresh, verify_refresh};
use crate::shared::errors::ApiError;
use crate::shared::extractors::DbConn;

use axum::{Json, extract::State, http::StatusCode};
use tower_cookies::{Cookie, Cookies};
use utoipa_axum::{router::OpenApiRouter, routes};

const ACCESS_TTL_SECONDS: i64 = 3600; // 1 hour
const REFRESH_TTL_SECONDS: i64 = 3600 * 24 * 30; // 30 days

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct RegisterRequest {
    pub username: String,
    pub email: String,
    pub display_name: String,
    pub password: String,
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct AccessTokenResponse {
    pub access_token: String,
    pub token_type: &'static str, // "Bearer"
    pub expires_in: i64,          // seconds
}

#[utoipa::path(
    post,
    path = "/register",
    tag = "auth",
    request_body = RegisterRequest,
    responses(
        (status = 200, description = "Registered user (disabled until an admin enables them)", body = User),
        (status = 400, description = "Invalid request, e.g. password shorter than 12 characters", body = ApiError),
        (status = 403, description = "User registration is disabled", body = ApiError),
        (status = 409, description = "Email address is already registered or username is taken", body = ApiError),
    )
)]
pub async fn register(
    DbConn(mut db): DbConn,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<User>, ApiError> {
    let inserted = register_user(
        &mut db,
        req.username,
        req.email,
        req.display_name,
        req.password,
    )
    .await?;

    Ok(Json(inserted))
}

pub fn issue_tokens(
    state: &AppState,
    cookies: &Cookies,
    user: &User,
) -> Result<AccessTokenResponse, ApiError> {
    let access_token = sign_access(
        &state.jwt_secret,
        user.id,
        user.role,
        user.force_password_change,
        ACCESS_TTL_SECONDS,
    )
    .map_err(|_| ApiError::internal_server_error("Token error"))?;

    let refresh_token = sign_refresh(
        &state.jwt_secret,
        user.id,
        user.role,
        user.force_password_change,
        REFRESH_TTL_SECONDS,
    )
    .map_err(|_| ApiError::internal_server_error("Token error"))?;

    let mut cookie = Cookie::new("refresh_token", refresh_token);
    cookie.set_http_only(true);
    cookie.set_secure(is_production());
    cookie.set_same_site(tower_cookies::cookie::SameSite::Lax);
    cookie.set_path("/api/v1/auth");
    cookies.add(cookie);

    Ok(AccessTokenResponse {
        access_token,
        token_type: "Bearer",
        expires_in: ACCESS_TTL_SECONDS,
    })
}

#[utoipa::path(
    post,
    path = "/login",
    tag = "auth",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Access token; a refresh_token is set as an HTTP-only cookie scoped to /api/v1/auth", body = AccessTokenResponse),
        (status = 401, description = "Invalid credentials", body = ApiError),
    )
)]
pub async fn login(
    State(state): State<Arc<AppState>>,
    cookies: Cookies,
    DbConn(mut db): DbConn,
    Json(req): Json<LoginRequest>,
) -> Result<Json<AccessTokenResponse>, ApiError> {
    // 1) verify credentials
    let user = find_user_for_login(&mut db, &req.username).await?;

    let fail = || ApiError::unauthorized("Invalid credentials");
    let Some(user) = user else {
        return Err(fail());
    };
    validate_login_user(&user, &req.password)?;

    Ok(Json(issue_tokens(&state, &cookies, &user)?))
}

#[utoipa::path(
    post,
    path = "/logout",
    tag = "auth",
    responses(
        (status = 204, description = "Logged out; the refresh-token cookie is cleared"),
        (status = 500, description = "Internal error", body = ApiError),
    )
)]
pub async fn logout(cookies: Cookies) -> Result<StatusCode, ApiError> {
    // Clear the refresh token cookie
    let mut cookie = Cookie::new("refresh_token", "");
    cookie.set_http_only(true);
    cookie.set_secure(is_production());
    cookie.set_same_site(tower_cookies::cookie::SameSite::Lax);
    cookie.set_path("/api/v1/auth");
    cookie.set_max_age(tower_cookies::cookie::time::Duration::ZERO);
    cookies.add(cookie);

    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/refresh",
    tag = "auth",
    responses(
        (status = 200, description = "Fresh access token; requires the refresh_token cookie and rotates it", body = AccessTokenResponse),
        (status = 401, description = "Missing or invalid refresh token", body = ApiError),
    )
)]
pub async fn refresh(
    State(state): State<Arc<AppState>>,
    cookies: Cookies,
    DbConn(mut db): DbConn,
) -> Result<Json<AccessTokenResponse>, ApiError> {
    let refresh_cookie = cookies
        .get("refresh_token")
        .ok_or_else(|| ApiError::unauthorized("Missing refresh token"))?;

    let claims = verify_refresh(&state.jwt_secret, refresh_cookie.value())
        .map_err(|_| ApiError::unauthorized("Invalid refresh token"))?;

    let user = find_user_for_refresh(&mut db, claims.uid).await?;

    validate_refresh_user(&user)?;

    Ok(Json(issue_tokens(&state, &cookies, &user)?))
}

pub fn routes() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(register))
        .routes(routes!(login))
        .routes(routes!(refresh))
        .routes(routes!(logout))
}
