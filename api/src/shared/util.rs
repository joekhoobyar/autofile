use axum::{
    response::{IntoResponse, Response},
    http::StatusCode,
    Json,
};
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
    #[serde(skip_serializing)]
    pub status: StatusCode,
    pub message: String,
}

impl ApiError {
    pub fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
        }
    }

    pub fn internal_server_error(message: &str) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, message)
    }

    pub fn bad_request(message: &str) -> Self {
        Self::new(StatusCode::BAD_REQUEST, message)
    }

    pub fn not_found(message: &str) -> Self {
        Self::new(StatusCode::NOT_FOUND, message)
    }

    pub fn unauthorized(message: &str) -> Self {
        Self::new(StatusCode::UNAUTHORIZED, message)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(self)).into_response()
    }
}

// Map a Diesel error to an appropriate HTTP status code.
pub fn diesel_to_http(e: DieselError) -> StatusCode {
    eprintln!("Diesel error: {:?}", e);
    match e {
        DieselError::NotFound => StatusCode::NOT_FOUND,
        DieselError::DatabaseError(DatabaseErrorKind::UniqueViolation, _) => StatusCode::CONFLICT,
        DieselError::DatabaseError(DatabaseErrorKind::NotNullViolation, _) => StatusCode::UNPROCESSABLE_ENTITY,
        DieselError::DatabaseError(DatabaseErrorKind::ForeignKeyViolation, _) => StatusCode::UNPROCESSABLE_ENTITY,
        DieselError::DatabaseError(DatabaseErrorKind::CheckViolation, _) => StatusCode::UNPROCESSABLE_ENTITY,
        _ => StatusCode::BAD_REQUEST,
    }
}

// Deserialize an Option<Option<T>> where the outer Option indicates presence of the field,
// and the inner Option is the actual value (Some or None).
pub fn de_present_option<'de, D, T>(d: D) -> Result<Option<Option<T>>, D::Error>
where
    D: serde::de::Deserializer<'de>,
    T: serde::Deserialize<'de>,
{
    use serde::Deserialize;
    // If the field is present:
    // - null -> Option<T>::None
    // - value -> Option<T>::Some(value)
    // Then we wrap it in Some(...) to record presence.
    Ok(Some(<Option<T>>::deserialize(d)?))
}
