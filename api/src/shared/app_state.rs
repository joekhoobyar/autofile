use std::sync::Arc;

use diesel_async::{pooled_connection::bb8, AsyncPgConnection};
use apalis_redis::RedisStorage;

use crate::application::document_thumbnails::GenerateThumbnail;

// Shared application state
#[derive(Clone)]
pub struct AppState {
    pub db_pool: bb8::Pool<AsyncPgConnection>,
    pub s3_client: Arc<aws_sdk_s3::Client>,
    pub s3_bucket: Arc<String>,
    pub jwt_secret: Arc<Vec<u8>>,
    pub thumb_jobs: Arc<RedisStorage<GenerateThumbnail>>,
}
