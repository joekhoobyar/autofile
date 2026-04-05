use std::sync::Arc;

use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
};
use axum::{
    extract::FromRequestParts,
    http::{StatusCode, request::Parts},
};
use chrono::Utc;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};

use crate::{shared::app_state::AppState, shared::util::ApiError};

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

#[derive(Debug, Clone, Copy)]
pub struct AuthUser {
    pub user_id: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AccessClaims {
    pub uid: i64,    // your user id
    pub exp: usize,  // unix timestamp
    pub iat: usize,  // unix timestamp
    pub iss: String, // issuer (optional but recommended)
    pub aud: String, // audience (optional but recommended)
    pub typ: String, // "access"
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RefreshClaims {
    pub uid: i64,    // your user id
    pub exp: usize,  // unix timestamp
    pub iat: usize,  // unix timestamp
    pub iss: String, // issuer (optional but recommended)
    pub aud: String, // audience (optional but recommended)
    pub typ: String, // "refresh"
}

pub fn sign_access(
    secret: &[u8],
    uid: i64,
    ttl_seconds: i64,
) -> jsonwebtoken::errors::Result<String> {
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

    jsonwebtoken::encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(secret),
    )
}

pub fn sign_refresh(
    secret: &[u8],
    uid: i64,
    ttl_seconds: i64,
) -> jsonwebtoken::errors::Result<String> {
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

    jsonwebtoken::encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(secret),
    )
}

pub fn verify_access(
    secret: &[u8],
    token: &str,
) -> Result<AccessClaims, jsonwebtoken::errors::Error> {
    let mut v = Validation::new(Algorithm::HS256);
    v.set_issuer(&[ISS]);
    v.set_audience(&[AUD]);
    v.validate_exp = true;

    let data = jsonwebtoken::decode::<AccessClaims>(token, &DecodingKey::from_secret(secret), &v)?;
    if data.claims.typ != "access" {
        return Err(jsonwebtoken::errors::Error::from(
            jsonwebtoken::errors::ErrorKind::InvalidToken,
        ));
    }
    Ok(data.claims)
}

pub fn verify_refresh(
    secret: &[u8],
    token: &str,
) -> Result<RefreshClaims, jsonwebtoken::errors::Error> {
    let mut v = Validation::new(Algorithm::HS256);
    v.set_issuer(&[ISS]);
    v.set_audience(&[AUD]);
    v.validate_exp = true;

    let data = jsonwebtoken::decode::<RefreshClaims>(token, &DecodingKey::from_secret(secret), &v)?;
    if data.claims.typ != "refresh" {
        return Err(jsonwebtoken::errors::Error::from(
            jsonwebtoken::errors::ErrorKind::InvalidToken,
        ));
    }
    Ok(data.claims)
}

impl FromRequestParts<Arc<AppState>> for AuthUser {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        // Extract Authorization header
        let auth_header = parts
            .headers
            .get("Authorization")
            .and_then(|h| h.to_str().ok())
            .ok_or_else(|| {
                ApiError::new(StatusCode::UNAUTHORIZED, "Missing Authorization header")
            })?;

        // Extract Bearer token
        let token = auth_header.strip_prefix("Bearer ").ok_or_else(|| {
            ApiError::new(
                StatusCode::UNAUTHORIZED,
                "Invalid Authorization header format",
            )
        })?;

        if token.is_empty() {
            return Err(ApiError::new(StatusCode::UNAUTHORIZED, "Empty token"));
        }

        // Verify the JWT
        match verify_access(&state.jwt_secret, token) {
            Ok(claims) => Ok(AuthUser {
                user_id: claims.uid,
            }),
            Err(_) => Err(ApiError::new(
                StatusCode::UNAUTHORIZED,
                "Invalid or expired token",
            )),
        }
    }
}
