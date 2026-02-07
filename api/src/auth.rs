use serde::{Deserialize, Serialize};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, Algorithm};
use chrono::Utc;
use rocket::{request::{FromRequest, Outcome}, http::Status, Request, State};
use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use rand::rngs::OsRng;

const ISS: &str = "autofile-api";
const AUD: &str = "autofile-spa";

pub fn hash_password(password: &str) -> Result<String, &'static str> {
    if password.len() < 12 {
        return Err("password too short");
    }

    let salt = SaltString::generate(&mut OsRng);

    // Argon2id with reasonable defaults; you can tune parameters later.
    let argon2 = Argon2::default();

    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|_| "hashing failed")?
        .to_string();

    Ok(hash)
}

pub fn verify_password(password: &str, password_hash: &str) -> Result<bool, &'static str> {
    let parsed = PasswordHash::new(password_hash).map_err(|_| "bad hash")?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok())
}

pub struct JwtSecret(pub Vec<u8>);

#[derive(Debug, Clone, Copy)]
pub struct AuthUser {
    pub user_id: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AccessClaims {
    pub uid: i64,          // your user id
    pub exp: usize,        // unix timestamp
    pub iat: usize,        // unix timestamp
    pub iss: String,       // issuer (optional but recommended)
    pub aud: String,       // audience (optional but recommended)
    pub typ: String,       // "access"
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RefreshClaims {
    pub uid: i64,          // your user id
    pub exp: usize,        // unix timestamp
    pub iat: usize,        // unix timestamp
    pub iss: String,       // issuer (optional but recommended)
    pub aud: String,       // audience (optional but recommended)
    pub typ: String,       // "refresh"
}

pub fn sign_access(secret: &[u8], uid: i64, ttl_seconds: i64) -> jsonwebtoken::errors::Result<String> {
    let now = Utc::now().timestamp() as usize;
    let exp = (Utc::now().timestamp() + ttl_seconds) as usize;

    let claims = AccessClaims {
        uid,
        iat: now,
        exp,
        iss: ISS.to_string(),
        aud: AUD.to_string(),
        typ: "access".into(),
    };

    jsonwebtoken::encode(&Header::new(Algorithm::HS256), &claims, &EncodingKey::from_secret(secret))
}

pub fn sign_refresh(secret: &[u8], uid: i64, ttl_seconds: i64) -> jsonwebtoken::errors::Result<String> {
    let now = Utc::now().timestamp() as usize;
    let exp = (Utc::now().timestamp() + ttl_seconds) as usize;

    let claims = RefreshClaims {
        uid,
        iat: now,
        exp,
        iss: ISS.to_string(),
        aud: AUD.to_string(),
        typ: "refresh".into(),
    };

    jsonwebtoken::encode(&Header::new(Algorithm::HS256), &claims, &EncodingKey::from_secret(secret))
}

pub fn verify_access(secret: &[u8], token: &str) -> Result<AccessClaims, jsonwebtoken::errors::Error> {
    let mut v = Validation::new(Algorithm::HS256);
    v.set_issuer(&[ISS]);
    v.set_audience(&[AUD]);
    v.validate_exp = true;

    let data = jsonwebtoken::decode::<AccessClaims>(
        token,
        &DecodingKey::from_secret(secret),
        &v,
    )?;
    if data.claims.typ != "access" {
        return Err(jsonwebtoken::errors::Error::from(
            jsonwebtoken::errors::ErrorKind::InvalidToken,
        ));
    }
    Ok(data.claims)
}

pub fn verify_refresh(secret: &[u8], token: &str) -> Result<RefreshClaims, jsonwebtoken::errors::Error> {
    let mut v = Validation::new(Algorithm::HS256);
    v.set_issuer(&[ISS]);
    v.set_audience(&[AUD]);
    v.validate_exp = true;

    let data = jsonwebtoken::decode::<RefreshClaims>(
        token,
        &DecodingKey::from_secret(secret),
        &v,
    )?;
    if data.claims.typ != "refresh" {
        return Err(jsonwebtoken::errors::Error::from(
            jsonwebtoken::errors::ErrorKind::InvalidToken,
        ));
    }
    Ok(data.claims)
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AuthUser {
    type Error = ();

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let secret = match req.guard::<&State<JwtSecret>>().await {
            Outcome::Success(s) => s,
            _ => return Outcome::Error((Status::InternalServerError, ())),
        };

        let auth = match req.headers().get_one("Authorization") {
            Some(h) => h,
            None => return Outcome::Error((Status::Unauthorized, ())),
        };

        let token = auth.strip_prefix("Bearer ").unwrap_or("");
        if token.is_empty() {
            return Outcome::Error((Status::Unauthorized, ()));
        }

        match verify_access(&secret.0, token) {
            Ok(data) => Outcome::Success(AuthUser { user_id: data.uid }),
            Err(_) => Outcome::Error((Status::Unauthorized, ())),
        }
    }
}
