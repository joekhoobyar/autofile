use std::sync::Arc;

use axum::{
    extract::FromRequestParts,
    http::request::Parts,
};
use diesel_async::{
    pooled_connection::bb8::PooledConnection,
    AsyncPgConnection,
};

use crate::{AppState, util::ApiError};

// Database connection extractor
pub struct DbConn(pub PooledConnection<'static, AsyncPgConnection>);

impl FromRequestParts<Arc<AppState>> for DbConn {
    type Rejection = ApiError;

    async fn from_request_parts(
        _parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        let conn = state
            .db_pool
            .get_owned()
            .await
            .map_err(|e| ApiError::internal_server_error(&format!("Failed to get DB connection: {}", e)))?;

        Ok(DbConn(conn))
    }
}
