use std::sync::Arc;

use crate::{AppState, is_production};
use crate::schema::users;
use crate::domain::users::User;
use crate::shared::auth::{hash_password, sign_access, sign_refresh, verify_password, verify_refresh};
use crate::shared::extractors::DbConn;
use crate::shared::util::{ApiError, diesel_to_http};

use axum::{
    Router,
    routing::post,
    Json,
    http::StatusCode,
    extract::State,
};
use tower_cookies::{Cookie, Cookies};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use chrono::Utc;

const ACCESS_TTL_SECONDS: i64 = 3600; // 1 hour
const REFRESH_TTL_SECONDS: i64 = 3600 * 24 * 30; // 30 days

#[derive(serde::Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub email: String,
    pub display_name: String,
    pub password: String,
}

#[derive(serde::Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(serde::Serialize)]
pub struct AccessTokenResponse {
    pub access_token: String,
    pub token_type: &'static str, // "Bearer"
    pub expires_in: i64,          // seconds
}

pub async fn register(
    DbConn(mut db): DbConn,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<User>, ApiError> {
    let pw_hash = hash_password(&req.password)
        .map_err(|m| ApiError::bad_request(m))?;

    let inserted: User = diesel::insert_into(users::table)
        .values((
            users::username.eq(&req.username),
            users::email.eq(&req.email),
            users::display_name.eq(&req.display_name),
            users::password_hash.eq(pw_hash),
            users::password_changed_at.eq(Utc::now()),
        ))
        .returning(User::as_returning())
        .get_result(&mut db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to register user"))?;

    Ok(Json(inserted))
}

pub async fn login(
    State(state): State<Arc<AppState>>,
    cookies: Cookies,
    DbConn(mut db): DbConn,
    Json(req): Json<LoginRequest>,
) -> Result<Json<AccessTokenResponse>, ApiError> {

    // 1) verify credentials
    let user = users::table
        .filter(users::username.eq(&req.username))
        .select(User::as_select())
        .first::<User>(&mut db)
        .await
        .ok();

    let fail = || ApiError::unauthorized("Invalid credentials");
    let Some(user) = user else {
        return Err(fail());
    };
    let ok = verify_password(&req.password, &user.password_hash).unwrap_or(false);
    if !ok {
        return Err(fail());
    }

    // 2) issue access jwt
    let access_token = sign_access(&state.jwt_secret, user.id, ACCESS_TTL_SECONDS)
        .map_err(|_| ApiError::internal_server_error("Token error"))?;

    // 3) issue refresh token + set cookie
    let refresh_token = sign_refresh(&state.jwt_secret, user.id, REFRESH_TTL_SECONDS)
        .map_err(|_| ApiError::internal_server_error("Token error"))?;

    let mut cookie = Cookie::new("refresh_token", refresh_token);
    cookie.set_http_only(true);
    cookie.set_secure(is_production());
    cookie.set_same_site(tower_cookies::cookie::SameSite::Lax);
    cookie.set_path("/auth");
    cookies.add(cookie);

    Ok(Json(AccessTokenResponse {
        access_token,
        token_type: "Bearer",
        expires_in: ACCESS_TTL_SECONDS
    }))
}

pub async fn logout(
    cookies: Cookies,
) -> Result<StatusCode, ApiError> {

    // Clear the refresh token cookie
    let mut cookie = Cookie::new("refresh_token", "");
    cookie.set_http_only(true);
    cookie.set_secure(is_production());
    cookie.set_same_site(tower_cookies::cookie::SameSite::Lax);
    cookie.set_path("/auth");
    cookie.set_max_age(tower_cookies::cookie::time::Duration::ZERO);
    cookies.add(cookie);

    Ok(StatusCode::NO_CONTENT)
}

pub async fn refresh(
    State(state): State<Arc<AppState>>,
    cookies: Cookies,
) -> Result<Json<AccessTokenResponse>, ApiError> {
    let refresh_cookie = cookies
        .get("refresh_token")
        .ok_or_else(|| ApiError::unauthorized("Missing refresh token"))?;

    let claims = verify_refresh(&state.jwt_secret, refresh_cookie.value())
        .map_err(|_| ApiError::unauthorized("Invalid refresh token"))?;

    let access = sign_access(&state.jwt_secret, claims.uid, ACCESS_TTL_SECONDS)
        .map_err(|_| ApiError::internal_server_error("Token error"))?;

    Ok(Json(AccessTokenResponse {
        access_token: access,
        token_type: "Bearer",
        expires_in: ACCESS_TTL_SECONDS,
    }))
}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/refresh", post(refresh))
        .route("/logout", post(logout))
}
