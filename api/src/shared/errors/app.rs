use std::fmt;

use anyhow::Error as AnyhowError;
use diesel::result::{DatabaseErrorKind, Error as DieselError};

use crate::infrastructure::s3::S3Error;

#[derive(Debug, Clone, Copy)]
pub enum AppErrorKind {
    InvalidInput,
    MissingResource,
    DuplicateResource,
    ConstraintViolation,
    DependencyFailure,
    Unexpected,
}

#[derive(Debug)]
pub struct AppError {
    pub kind: AppErrorKind,
    pub message: String,
    pub(crate) source: Option<AnyhowError>,
}

pub type AppResult<T> = Result<T, AppError>;

impl AppError {
    pub fn new(kind: AppErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            source: None,
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(AppErrorKind::Unexpected, message)
    }

    pub fn from_diesel(message: impl Into<String>, err: DieselError) -> Self {
        Self {
            kind: AppErrorKind::from(&err),
            message: message.into(),
            source: Some(AnyhowError::new(err)),
        }
    }

    pub fn with_source<E>(kind: AppErrorKind, message: impl Into<String>, err: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self {
            kind,
            message: message.into(),
            source: Some(AnyhowError::new(err)),
        }
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(source) = &self.source {
            write!(f, "{}: {}", self.message, source)
        } else {
            write!(f, "{}", self.message)
        }
    }
}

impl std::error::Error for AppError {}

impl From<&DieselError> for AppErrorKind {
    fn from(err: &DieselError) -> Self {
        match err {
            DieselError::NotFound => AppErrorKind::MissingResource,
            DieselError::DatabaseError(DatabaseErrorKind::UniqueViolation, _) => {
                AppErrorKind::DuplicateResource
            }
            DieselError::DatabaseError(DatabaseErrorKind::NotNullViolation, _) => {
                AppErrorKind::ConstraintViolation
            }
            DieselError::DatabaseError(DatabaseErrorKind::ForeignKeyViolation, _) => {
                AppErrorKind::ConstraintViolation
            }
            DieselError::DatabaseError(DatabaseErrorKind::CheckViolation, _) => {
                AppErrorKind::ConstraintViolation
            }
            _ => AppErrorKind::Unexpected,
        }
    }
}

impl From<S3Error> for AppError {
    fn from(err: S3Error) -> Self {
        AppError::with_source(
            AppErrorKind::DependencyFailure,
            "Storage operation failed",
            err,
        )
    }
}

pub trait AppErrorContext<T> {
    fn app_context(self, message: &'static str) -> AppResult<T>;
}

impl<T> AppErrorContext<T> for Result<T, DieselError> {
    fn app_context(self, message: &'static str) -> AppResult<T> {
        self.map_err(|err| AppError::from_diesel(message, err))
    }
}
