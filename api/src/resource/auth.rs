use crate::{Db, is_production};
use crate::auth::{JwtSecret, hash_password, sign_access, sign_refresh, verify_password, verify_refresh};
use crate::util::{ApiResult, SameOrigin, diesel_to_http, err};
use crate::schema::users;
use crate::resource::users::{User};

use rocket::State;
use rocket::http::{Cookie, CookieJar, SameSite, Status};
use rocket::serde::json::Json;

use rocket_db_pools::Connection;
use rocket_db_pools::diesel::prelude::*;

use chrono::Utc;

const ACCESS_TTL_SECONDS: i64 = 3600; // 1 hour
const REFRESH_TTL_SECONDS: i64 = 3600 * 24 * 30; // 30 days

#[derive(rocket::serde::Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct RegisterRequest {
    pub username: String,
    pub email: String,
    pub display_name: String,
    pub password: String,
}

#[derive(rocket::serde::Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(rocket::serde::Serialize)]
#[serde(crate = "rocket::serde")]
pub struct AccessTokenResponse {
    pub access_token: String,
    pub token_type: &'static str, // "Bearer"
    pub expires_in: i64,          // seconds
}


#[post("/register", format="json", data="<req>")]
pub async fn register(
    mut db: Connection<Db>,
    req: Json<RegisterRequest>,
) -> ApiResult<Json<User>> {
    let pw_hash = hash_password(&req.password)
        .map_err(|m| err(Status::BadRequest, m))?;

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
        .map_err(|e| err(diesel_to_http(e), "failed to register user"))?;

    Ok(Json(inserted))
}

#[post("/login", format="json", data="<req>")]
pub async fn login(
    mut db: Connection<Db>,
    req: Json<LoginRequest>,
    cookies: &CookieJar<'_>,
    jwt: &State<JwtSecret>,
) -> ApiResult<Json<AccessTokenResponse>> {

    // 1) verify credentials
    let user = users::table
        .filter(users::username.eq(&req.username))
        .select(User::as_select())
        .first::<User>(&mut db)
        .await
        .ok();
    let fail = || err(Status::Unauthorized, "invalid credentials");
    let Some(user) = user else {
        return Err(fail());
    };
    let ok = verify_password(&req.password, &user.password_hash).unwrap_or(false);
    if !ok {
        return Err(fail());
    }

    // 2) issue access jwt
    let access_token = sign_access(&jwt.0, user.id, ACCESS_TTL_SECONDS)
        .map_err(|_| err(Status::InternalServerError, "token error"))?;

    // 3) issue refresh token + set cookie
    let refresh_token = sign_refresh(&jwt.0, user.id, REFRESH_TTL_SECONDS)
        .map_err(|_| err(Status::InternalServerError, "token error"))?;
    let cookie = Cookie::build(("refresh_token", refresh_token))
        .http_only(true)
        .secure(is_production())          // require HTTPS in prod
        .same_site(SameSite::Lax)
        .path("/auth")         // restrict where the cookie is sent
        .build();
    cookies.add(cookie);

    // success: issue tokens (JWT + refresh token)
    // ...
    Ok(Json(AccessTokenResponse { access_token, token_type: "Bearer", expires_in: ACCESS_TTL_SECONDS }))
}

#[post("/auth/logout")]
async fn logout(
    _origin_ok: SameOrigin,
    cookies: &CookieJar<'_>,
) -> ApiResult<Status> {

    // Clear the refresh token cookie
    let cookie = Cookie::build(("refresh_token", ""))
        .http_only(true)
        .secure(is_production())
        .same_site(SameSite::Lax)
        .path("/auth")
        .max_age(rocket::time::Duration::seconds(0))
        .build();
    cookies.add(cookie);

    Ok(Status::NoContent)
}

#[post("/auth/refresh")]
async fn refresh(
    _origin_ok: SameOrigin,
    cookies: &CookieJar<'_>,
    jwt: &rocket::State<JwtSecret>,
) -> ApiResult<rocket::serde::json::Json<AccessTokenResponse>> {
    let refresh_cookie = cookies
        .get("refresh_token")
        .ok_or_else(|| err(Status::Unauthorized, "missing refresh token"))?;

    let claims = verify_refresh(&jwt.0, refresh_cookie.value())
        .map_err(|_| err(Status::Unauthorized, "invalid refresh token"))?;

    let access = sign_access(&jwt.0, claims.uid, ACCESS_TTL_SECONDS)
        .map_err(|_| err(Status::InternalServerError, "token error"))?;

    Ok(rocket::serde::json::Json(AccessTokenResponse {
        access_token: access,
        token_type: "Bearer",
        expires_in: ACCESS_TTL_SECONDS,
    }))
}


pub fn stage() -> rocket::fairing::AdHoc {
    rocket::fairing::AdHoc::on_ignite("Autofile auth", |rocket| async {
        rocket.mount("/auth", routes![register, login, refresh, logout])
    })
}
