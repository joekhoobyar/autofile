use std::fmt;

use anyhow::Error as AnyhowError;
use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use diesel::result::Error as DieselError;

use crate::shared::errors::app::{AppError, AppErrorKind};

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct ApiError {
    #[serde(skip_serializing)]
    pub status: StatusCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<&'static str>,
    #[serde(skip)]
    #[schema(ignore)]
    source: Option<AnyhowError>,
}

pub type ApiResult<T> = Result<T, ApiError>;

impl ApiError {
    pub fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
            code: None,
            source: None,
        }
    }

    pub fn with_code(status: StatusCode, message: impl Into<String>, code: &'static str) -> Self {
        Self {
            status,
            message: message.into(),
            code: Some(code),
            source: None,
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

    pub fn conflict(message: &str) -> Self {
        Self::new(StatusCode::CONFLICT, message)
    }

    pub fn unprocessable_entity(message: &str) -> Self {
        Self::new(StatusCode::UNPROCESSABLE_ENTITY, message)
    }

    pub fn unauthorized(message: &str) -> Self {
        Self::new(StatusCode::UNAUTHORIZED, message)
    }

    pub fn from_diesel(message: impl Into<String>, err: DieselError) -> Self {
        Self {
            status: app_error_kind_status(AppErrorKind::from(&err)),
            message: message.into(),
            code: None,
            source: Some(AnyhowError::new(err)),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        if self.status.is_server_error() {
            if let Some(source) = &self.source {
                tracing::error!(
                    status = %self.status,
                    message = %self.message,
                    error = ?source,
                    "server error response"
                );
            } else {
                tracing::error!(
                    status = %self.status,
                    message = %self.message,
                    "server error response"
                );
            }
        }

        (self.status, Json(self)).into_response()
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ApiError {}

impl From<DieselError> for ApiError {
    fn from(err: DieselError) -> Self {
        let message = match &err {
            DieselError::NotFound => "Resource not found",
            _ => "Database query failed",
        };

        ApiError::from_diesel(message, err)
    }
}

impl From<AppError> for ApiError {
    fn from(err: AppError) -> Self {
        ApiError {
            status: app_error_kind_status(err.kind),
            message: err.message,
            code: None,
            source: err.source,
        }
    }
}

pub trait ApiErrorContext<T> {
    fn api_context(self, message: &'static str) -> ApiResult<T>;
}

impl<T> ApiErrorContext<T> for Result<T, DieselError> {
    fn api_context(self, message: &'static str) -> ApiResult<T> {
        self.map_err(|err| ApiError::from_diesel(message, err))
    }
}

fn app_error_kind_status(kind: AppErrorKind) -> StatusCode {
    match kind {
        AppErrorKind::InvalidInput => StatusCode::BAD_REQUEST,
        AppErrorKind::MissingResource => StatusCode::NOT_FOUND,
        AppErrorKind::DuplicateResource => StatusCode::CONFLICT,
        AppErrorKind::ConstraintViolation => StatusCode::UNPROCESSABLE_ENTITY,
        AppErrorKind::DependencyFailure | AppErrorKind::Unexpected => {
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}
