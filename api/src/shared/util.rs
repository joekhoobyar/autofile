use std::fmt;
use std::sync::Arc;

use apalis::prelude::*;
use anyhow::Error as AnyhowError;
use axum::{
    response::{IntoResponse, Response},
    http::StatusCode,
    Json,
};
use diesel::result::{DatabaseErrorKind, Error as DieselError};
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

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

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ApiError {}

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

pub struct TempUpload {
    pub path: std::path::PathBuf,
    pub size: i64,
}

#[derive(Debug)]
pub struct JobError(pub AnyhowError);

pub type JobResult<T> = Result<T, JobError>;

impl fmt::Display for JobError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<E> From<E> for JobError
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn from(err: E) -> Self {
        Self(AnyhowError::new(err))
    }
}

impl From<JobError> for Error {
    fn from(err: JobError) -> Self {
        to_job_error(err.0)
    }
}

pub async fn write_field_to_temp_file(
    field: &mut axum::extract::multipart::Field<'_>,
) -> Result<TempUpload, ApiError> {
    let mut temp_path = std::env::temp_dir();
    temp_path.push(format!("autofile-upload-{}", Uuid::new_v4()));
    let mut temp_file = tokio::fs::File::create(&temp_path)
        .await
        .map_err(|e| ApiError::internal_server_error(&format!("Failed to create temp file: {}", e)))?;

    let mut size: i64 = 0;
    loop {
        let chunk = field
            .chunk()
            .await
            .map_err(|e| ApiError::bad_request(&format!("Failed to read file data: {}", e)))?;

        let Some(chunk) = chunk else {
            break;
        };

        size += chunk.len() as i64;
        temp_file
            .write_all(&chunk)
            .await
            .map_err(|e| ApiError::internal_server_error(&format!("Failed to buffer upload: {}", e)))?;
    }

    temp_file
        .flush()
        .await
        .map_err(|e| ApiError::internal_server_error(&format!("Failed to finalize temp file: {}", e)))?;

    Ok(TempUpload {
        path: temp_path,
        size,
    })
}

/*
 * Convert any error into a job error that can be returned from an Apalis job.
 */
pub fn to_job_error<E>(err: E) -> Error
where
    E: std::error::Error + Send + Sync + 'static,
{
    let boxed: BoxDynError = Box::new(err);
    Error::Failed(Arc::new(boxed))
}
