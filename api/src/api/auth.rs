use std::sync::Arc;

use crate::application::app_settings::get_app_settings;
use crate::domain::users::{User, UserRole};
use crate::is_production;
use crate::schema::users;
use crate::shared::app_state::AppState;
use crate::shared::auth::{
    hash_password, sign_access, sign_refresh, verify_password, verify_refresh,
};
use crate::shared::extractors::DbConn;
use crate::shared::util::{ApiError, diesel_to_http};

use axum::{Json, Router, extract::State, http::StatusCode, routing::post};
use chrono::Utc;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use tower_cookies::{Cookie, Cookies};

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
    let settings = get_app_settings(&mut db).await?;
    if !settings.allow_user_registration {
        return Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "User registration is disabled",
        ));
    }

    let pw_hash = hash_password(&req.password).map_err(ApiError::bad_request)?;

    let inserted: User = diesel::insert_into(users::table)
        .values((
            users::username.eq(&req.username),
            users::email.eq(&req.email),
            users::display_name.eq(&req.display_name),
            users::password_hash.eq(pw_hash),
            users::password_changed_at.eq(Utc::now()),
            users::role.eq(UserRole::User),
            users::enabled.eq(false),
        ))
        .returning(User::as_returning())
        .get_result(&mut db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to register user"))?;

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
    validate_login_user(&user, &req.password)?;

    Ok(Json(issue_tokens(&state, &cookies, &user)?))
}

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

    let user = users::table
        .find(claims.uid)
        .select(User::as_select())
        .first::<User>(&mut db)
        .await
        .map_err(|_| ApiError::unauthorized("Invalid refresh token"))?;

    validate_refresh_user(&user)?;

    Ok(Json(issue_tokens(&state, &cookies, &user)?))
}

fn validate_login_user(user: &User, password: &str) -> Result<(), ApiError> {
    let fail = || ApiError::unauthorized("Invalid credentials");

    let ok = verify_password(password, &user.password_hash).unwrap_or(false);
    if !ok || !user.enabled {
        return Err(fail());
    }

    Ok(())
}

fn validate_refresh_user(user: &User) -> Result<(), ApiError> {
    if !user.enabled {
        return Err(ApiError::unauthorized("Invalid refresh token"));
    }

    Ok(())
}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/refresh", post(refresh))
        .route("/logout", post(logout))
}

#[cfg(test)]
mod tests {
    use super::{validate_login_user, validate_refresh_user};
    use crate::domain::users::{User, UserRole};
    use crate::shared::auth::hash_password;
    use axum::http::StatusCode;
    use chrono::Utc;

    fn test_user(enabled: bool) -> User {
        User {
            id: 42,
            username: "test-user".to_string(),
            email: "test-user@example.com".to_string(),
            display_name: "Test User".to_string(),
            password_hash: hash_password("long-enough-password").expect("hash should succeed"),
            password_changed_at: Utc::now(),
            role: UserRole::User,
            force_password_change: false,
            enabled,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn disabled_user_cannot_login() {
        let err = validate_login_user(&test_user(false), "long-enough-password")
            .expect_err("disabled user should not login");

        assert_eq!(err.status, StatusCode::UNAUTHORIZED);
        assert_eq!(err.message, "Invalid credentials");
    }

    #[test]
    fn disabled_user_cannot_refresh() {
        let err =
            validate_refresh_user(&test_user(false)).expect_err("disabled user should not refresh");

        assert_eq!(err.status, StatusCode::UNAUTHORIZED);
        assert_eq!(err.message, "Invalid refresh token");
    }

    #[test]
    fn enabled_user_can_login_and_refresh() {
        let user = test_user(true);

        validate_login_user(&user, "long-enough-password").expect("enabled user should login");
        validate_refresh_user(&user).expect("enabled user should refresh");
    }
}
