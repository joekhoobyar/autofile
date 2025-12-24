use rocket::{http::Status, response::Responder, Request};
use rocket::serde::json::Json;
use rocket::response::status::Custom;

use diesel::result::{DatabaseErrorKind, Error as DieselError};

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

pub fn diesel_to_http(e: DieselError) -> Status {
    match e {
        DieselError::NotFound => Status::NotFound,
        DieselError::DatabaseError(DatabaseErrorKind::UniqueViolation, _) => Status::Conflict,
        DieselError::DatabaseError(DatabaseErrorKind::ForeignKeyViolation, _) => Status::BadRequest,
        _ => {
            println!("Unhandled Diesel error: {:?}", e);
            Status::InternalServerError
        },
    }
}
