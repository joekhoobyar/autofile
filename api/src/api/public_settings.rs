use std::sync::Arc;

use axum::{Json, Router, routing::get};

use crate::application::app_settings::get_app_settings;
use crate::shared::app_state::AppState;
use crate::shared::extractors::DbConn;
use crate::shared::util::ApiError;

/// Settings that are safe to expose without authentication.
///
/// This is an explicit allowlist: new `AppSettings` fields are never
/// public unless they are deliberately added to this struct.
#[derive(serde::Serialize)]
pub struct PublicSettingsResponse {
    pub allow_user_registration: bool,
}

pub async fn get_public_settings(
    DbConn(mut db): DbConn,
) -> Result<Json<PublicSettingsResponse>, ApiError> {
    let settings = get_app_settings(&mut db).await?;

    Ok(Json(PublicSettingsResponse {
        allow_user_registration: settings.allow_user_registration,
    }))
}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/settings", get(get_public_settings))
}
