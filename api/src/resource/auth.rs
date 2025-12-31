use crate::Db;
use crate::auth::{JwtSecret, hash_password, sign_jwt, verify_password};
use crate::util::{ApiResult, diesel_to_http, err};
use crate::schema::users;
use crate::resource::users::{User};

use rocket::State;
use rocket::http::Status;
use rocket::serde::json::Json;

use rocket_db_pools::Connection;
use rocket_db_pools::diesel::prelude::*;

use chrono::{DateTime, Utc};

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
pub struct TokenResponse {
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
    jwt: &State<JwtSecret>,
) -> ApiResult<Json<TokenResponse>> {

    let user = users::table
        .filter(users::username.eq(&req.username))
        .select(User::as_select())
        .first::<User>(&mut db)
        .await
        .ok();

    // 1) verify credentials
    let fail = || err(Status::Unauthorized, "invalid credentials");
    let Some(user) = user else {
        return Err(fail());
    };
    let ok = verify_password(&req.password, &user.password_hash).unwrap_or(false);
    if !ok {
        return Err(fail());
    }

    // 2) issue access jwt
    let access_token = sign_jwt(&jwt.0, user.id /* uid */)
        .map_err(|_| err(Status::InternalServerError, "token error"))?;

    // success: issue tokens (JWT + refresh token)
    // ...
    Ok(Json(TokenResponse { access_token, token_type: "Bearer", expires_in: 3600  }))
}


pub fn stage() -> rocket::fairing::AdHoc {
    rocket::fairing::AdHoc::on_ignite("Autofile auth", |rocket| async {
        rocket.mount("/auth", routes![register, login])
    })
}
