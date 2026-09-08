use std::sync::Arc;

use axum::{Json, Router, routing::get};
use serde_json::{Value, json};

use crate::shared::app_state::AppState;
use crate::shared::auth::AuthUser;

async fn ping(_user: AuthUser) -> Json<Value> {
    Json(json!({}))
}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/", get(ping))
}
