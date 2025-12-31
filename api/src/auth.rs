use serde::{Deserialize, Serialize};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, Algorithm, TokenData};
use chrono::Utc;
use rocket::{request::{FromRequest, Outcome}, http::Status, Request, State};

pub struct JwtSecret(pub Vec<u8>);

#[derive(Debug, Clone, Copy)]
pub struct AuthUser {
    pub user_id: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub uid: i64,          // your user id
    pub exp: usize,        // unix timestamp
    pub iat: usize,        // unix timestamp
    pub iss: String,       // issuer (optional but recommended)
    pub aud: String,       // audience (optional but recommended)
}

pub fn sign_jwt(secret: &[u8], uid: i64) -> Result<String, jsonwebtoken::errors::Error> {
    let now = Utc::now().timestamp() as usize;
    let exp = now + 60 * 60; // 1 hour

    let claims = Claims {
        uid,
        iat: now,
        exp,
        iss: "autofile-api".to_string(),
        aud: "autofile".to_string(),
    };

    Ok(jsonwebtoken::encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(secret),
    )?)
}

pub fn verify_jwt(secret: &[u8], token: &str) -> Result<TokenData<Claims>, jsonwebtoken::errors::Error> {
    let mut v = Validation::new(Algorithm::HS256);
    v.set_issuer(&["autofile-api"]);
    v.set_audience(&["autofile"]);
    jsonwebtoken::decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret),
        &v,
    )
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

        match verify_jwt(&secret.0, token) {
            Ok(data) => Outcome::Success(AuthUser { user_id: data.claims.uid }),
            Err(_) => Outcome::Error((Status::Unauthorized, ())),
        }
    }
}
