use std::sync::Arc;

use axum::Json;
use serde_json::{Value, json};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::shared::app_state::AppState;
use crate::shared::auth::AuthUser;
use crate::shared::errors::ApiError;

#[utoipa::path(
    get,
    path = "/",
    tag = "ping",
    security(("bearer" = [])),
    responses(
        (status = 200, description = "Empty object; verifies the access token is accepted", body = Value),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
    )
)]
async fn ping(_user: AuthUser) -> Json<Value> {
    Json(json!({}))
}

pub fn routes() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new().routes(routes!(ping))
}
