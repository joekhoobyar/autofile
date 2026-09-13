use std::sync::Arc;

use apalis_redis::RedisStorage;
use diesel_async::{AsyncPgConnection, pooled_connection::bb8};

use crate::application::jobs::{FastJob, MediumJob, SlowJob};

// Shared application state
#[derive(Clone)]
pub struct AppState {
    pub db_pool: bb8::Pool<AsyncPgConnection>,
    pub s3_client: Arc<aws_sdk_s3::Client>,
    pub s3_bucket: Arc<String>,
    pub jwt_secret: Arc<Vec<u8>>,
    pub fast_jobs: Arc<RedisStorage<FastJob>>,
    pub medium_jobs: Arc<RedisStorage<MediumJob>>,
    pub slow_jobs: Arc<RedisStorage<SlowJob>>,
    /// Maximum size in bytes for a single uploaded file. Read once from
    /// `MAX_UPLOAD_SIZE_MB` at startup and enforced while streaming uploads.
    pub max_upload_bytes: usize,
}
