use std::fmt;

use anyhow::Error as AnyhowError;
use apalis::prelude::BoxDynError;

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

impl From<JobError> for BoxDynError {
    fn from(err: JobError) -> Self {
        Box::new(AnyhowJobError(err.0))
    }
}

#[derive(Debug)]
struct AnyhowJobError(AnyhowError);

impl fmt::Display for AnyhowJobError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for AnyhowJobError {}
