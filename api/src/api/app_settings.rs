use std::sync::Arc;

use axum::Json;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::application::app_settings::{
    UpdateAppSettingsInput, get_app_settings, update_app_settings,
};
use crate::domain::app_settings::AppSettings;
use crate::shared::app_state::AppState;
use crate::shared::auth::AdminUser;
use crate::shared::extractors::DbConn;
use crate::shared::util::ApiError;

#[utoipa::path(
    get,
    path = "/",
    tag = "settings",
    security(("bearer" = [])),
    responses(
        (status = 200, description = "Application settings", body = AppSettings),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Admin role required", body = ApiError),
    )
)]
pub async fn get_settings(
    _admin: AdminUser,
    DbConn(mut db): DbConn,
) -> Result<Json<AppSettings>, ApiError> {
    Ok(Json(get_app_settings(&mut db).await?))
}

#[utoipa::path(
    patch,
    path = "/",
    tag = "settings",
    security(("bearer" = [])),
    request_body = UpdateAppSettingsInput,
    responses(
        (status = 200, description = "Updated application settings", body = AppSettings),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Admin role required", body = ApiError),
        (status = 422, description = "Invalid request", body = ApiError),
    )
)]
pub async fn update_settings(
    _admin: AdminUser,
    DbConn(mut db): DbConn,
    Json(input): Json<UpdateAppSettingsInput>,
) -> Result<Json<AppSettings>, ApiError> {
    Ok(Json(update_app_settings(&mut db, input).await?))
}

pub fn routes() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(get_settings))
        .routes(routes!(update_settings))
}
