use rocket::{http::Status, response::Responder, Request};
use rocket::serde::json::Json;
use rocket::response::status::Custom;

use diesel::result::{DatabaseErrorKind, Error as DieselError};

#[derive(Debug)]
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
        let body = Json(ErrorBody { error: self.message });
        rocket::response::status::Custom(self.status, body).respond_to(rq)
        // NOTE: Rocket requires a Request; easiest in practice is to return Custom<Json<_>> directly.
    }
}

#[derive(serde::Serialize)]
pub struct ErrorBody {
    pub error: String,
}

pub type ApiResult<T> = Result<T, Custom<Json<ErrorBody>>>;

pub fn err(status: Status, msg: impl Into<String>) -> Custom<Json<ErrorBody>> {
    Custom(status, Json(ErrorBody { error: msg.into() }))
}

pub fn diesel_to_http(e: DieselError) -> Status {
    match e {
        DieselError::NotFound => Status::NotFound,
        DieselError::DatabaseError(DatabaseErrorKind::UniqueViolation, _) => Status::Conflict,
        DieselError::DatabaseError(DatabaseErrorKind::ForeignKeyViolation, _) => Status::BadRequest,
        _ => Status::InternalServerError,
    }
}
