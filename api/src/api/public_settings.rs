use std::sync::Arc;

use axum::Json;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::application::app_settings::get_app_settings;
use crate::shared::app_state::AppState;
use crate::shared::errors::ApiError;
use crate::shared::extractors::DbConn;

/// Settings that are safe to expose without authentication.
///
/// This is an explicit allowlist: new `AppSettings` fields are never
/// public unless they are deliberately added to this struct.
#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct PublicSettingsResponse {
    pub allow_user_registration: bool,
    /// Frontend-only date display and date-picker format. Date metadata API values remain YYYY-MM-DD.
    #[schema(example = "yyyy-MM-dd")]
    pub date_format: String,
    /// Frontend-only date/time display format for timestamps. API timestamps remain serialized as timestamp strings.
    #[schema(example = "MM/dd/yyyy HH:mm")]
    pub datetime_format: String,
}

#[utoipa::path(
    get,
    path = "/settings",
    tag = "public",
    responses(
        (status = 200, description = "Publicly visible settings", body = PublicSettingsResponse),
    )
)]
pub async fn get_public_settings(
    DbConn(mut db): DbConn,
) -> Result<Json<PublicSettingsResponse>, ApiError> {
    let settings = get_app_settings(&mut db).await?;

    Ok(Json(PublicSettingsResponse {
        allow_user_registration: settings.allow_user_registration,
        date_format: settings.date_format,
        datetime_format: settings.datetime_format,
    }))
}

pub fn routes() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new().routes(routes!(get_public_settings))
}
