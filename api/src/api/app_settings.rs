use std::sync::Arc;

use axum::{Json, Router, routing::get};

use crate::application::app_settings::{
    UpdateAppSettingsInput, get_app_settings, update_app_settings,
};
use crate::domain::app_settings::AppSettings;
use crate::shared::app_state::AppState;
use crate::shared::auth::AdminUser;
use crate::shared::extractors::DbConn;
use crate::shared::util::ApiError;

pub async fn get_settings(
    _admin: AdminUser,
    DbConn(mut db): DbConn,
) -> Result<Json<AppSettings>, ApiError> {
    Ok(Json(get_app_settings(&mut db).await?))
}

pub async fn update_settings(
    _admin: AdminUser,
    DbConn(mut db): DbConn,
    Json(input): Json<UpdateAppSettingsInput>,
) -> Result<Json<AppSettings>, ApiError> {
    Ok(Json(update_app_settings(&mut db, input).await?))
}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/", get(get_settings).patch(update_settings))
}
