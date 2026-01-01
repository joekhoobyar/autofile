use crate::{OurAllowedOrigins, is_production};

use rocket::State;
use rocket::request::{FromRequest, Outcome};
use rocket::{http::Status, response::Responder, Request};
use rocket::serde::json::Json;
use rocket::serde::Deserialize;
use rocket::response::status::Custom;
use rocket::form::{self, FromFormField, ValueField};
use rocket::http::{Cookie, CookieJar, SameSite};

use diesel::result::{DatabaseErrorKind, Error as DieselError};

#[derive(Debug, serde::Serialize)]
pub struct ResourceList<T> {
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
    pub items: Vec<T>,
}

#[derive(Debug, serde::Serialize)]
pub struct ApiError {
    pub status: Status,
    pub message: String,
}

impl ApiError {
    pub fn new(status: Status, message: impl Into<String>) -> Self {
        Self { status, message: message.into() }
    }
}

impl<'r> Responder<'r, 'static> for ApiError {
    fn respond_to(self, rq: &'r Request<'_>) -> rocket::response::Result<'static> {
        let status = self.status;
        rocket::response::status::Custom(status, Json(self)).respond_to(rq)
        // NOTE: Rocket requires a Request; easiest in practice is to return Custom<Json<_>> directly.
    }
}

pub type ApiResult<T> = Result<T, Custom<Json<ApiError>>>;

pub fn err(status: Status, msg: impl Into<String>) -> Custom<Json<ApiError>> {
    Custom(status, Json(ApiError::new(status, msg.into())))
}

// Map a Diesel error to an appropriate HTTP status code.
pub fn diesel_to_http(e: DieselError) -> Status {
    println!("Diesel error: {:?}", e);
    match e {
        DieselError::NotFound => Status::NotFound,
        DieselError::DatabaseError(DatabaseErrorKind::UniqueViolation, _) => Status::Conflict,
        DieselError::DatabaseError(DatabaseErrorKind::NotNullViolation, _) => Status::UnprocessableEntity,
        DieselError::DatabaseError(DatabaseErrorKind::ForeignKeyViolation, _) => Status::UnprocessableEntity,
        DieselError::DatabaseError(DatabaseErrorKind::CheckViolation, _) => Status::UnprocessableEntity,
        _ => {
            Status::BadRequest
        },
    }
}

// A form field to use when distinguishing between "not present" and "present but empty".
#[derive(Debug, Clone, Copy)]
pub enum FormFieldPresence<T> {
    Null,      // param present but empty
    Value(T),
}

impl<'v, T> FromFormField<'v> for FormFieldPresence<T>
where
    T: FromFormField<'v>,
{
    fn from_value(field: ValueField<'v>) -> form::Result<'v, Self> {
        if field.value.is_empty() {
            Ok(FormFieldPresence::Null)
        } else {
            Ok(FormFieldPresence::Value(T::from_value(field)?))
        }
    }
}

// Deserialize an Option<Option<T>> where the outer Option indicates presence of the field,
// and the inner Option is the actual value (Some or None).
pub fn de_present_option<'de, D, T>(d: D) -> Result<Option<Option<T>>, D::Error>
where
    D: serde::de::Deserializer<'de>,
    T: serde::Deserialize<'de>,
{
    // If the field is present:
    // - null -> Option<T>::None
    // - value -> Option<T>::Some(value)
    // Then we wrap it in Some(...) to record presence.
    Ok(Some(Option::<T>::deserialize(d)?))
}

pub struct SameOrigin;

// Require that the request's Origin header matches the allowed origins.
#[rocket::async_trait]
impl<'r> FromRequest<'r> for SameOrigin {
    type Error = ();

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let allowed = match req.guard::<&State<OurAllowedOrigins>>().await {
            Outcome::Success(s) => s,
            _ => return Outcome::Error((Status::InternalServerError, ())),
        };

        let origin = match req.headers().get_one("Origin") {
            Some(o) => o,
            None => return Outcome::Error((Status::Forbidden, ())), // no Origin => reject
        };

        if allowed.0.iter().any(|a| a == origin) {
            Outcome::Success(SameOrigin)
        } else {
            Outcome::Error((Status::Forbidden, ()))
        }
    }
}