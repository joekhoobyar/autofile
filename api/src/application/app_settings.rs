use bb8::PooledConnection;
use diesel::prelude::*;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::{AsyncPgConnection, RunQueryDsl};

use crate::domain::app_settings::AppSettings;
use crate::schema::app_settings;
use crate::shared::errors::{ApiError, ApiErrorContext};

const APP_SETTINGS_ID: i64 = 1;

const ALLOWED_DATE_FORMATS: &[&str] = &[
    "yyyy-MM-dd",
    "MM/dd/yyyy",
    "dd/MM/yyyy",
    "dd.MM.yyyy",
    "MMM d, yyyy",
];

const ALLOWED_DATETIME_FORMATS: &[&str] = &[
    "MM/dd/yyyy HH:mm",
    "MM/dd/yyyy h:mm a",
    "yyyy-MM-dd HH:mm",
    "dd/MM/yyyy HH:mm",
    "dd.MM.yyyy HH:mm",
    "MMM d, yyyy h:mm a",
];

#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct UpdateAppSettingsInput {
    pub allow_user_registration: bool,
    /// Frontend-only date display and date-picker format. Must be one of the supported UI date formats. Date metadata API values remain YYYY-MM-DD.
    #[schema(example = "yyyy-MM-dd")]
    pub date_format: String,
    /// Frontend-only date/time display format for timestamps. Must be one of the supported UI date/time formats. API timestamps remain serialized as timestamp strings.
    #[schema(example = "MM/dd/yyyy HH:mm")]
    pub datetime_format: String,
    /// Master switch for virus scanning. When false, uploads skip scanning and scan UI is hidden.
    pub virus_scanning_enabled: bool,
    /// Default checked state for the per-upload `Virus scan this upload` checkbox. Only meaningful when `virus_scanning_enabled` is true.
    pub virus_scan_by_default: bool,
}

pub async fn get_app_settings(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
) -> Result<AppSettings, ApiError> {
    app_settings::table
        .find(APP_SETTINGS_ID)
        .select(AppSettings::as_select())
        .first::<AppSettings>(db)
        .await
        .api_context("Failed to fetch app settings")
}

pub async fn update_app_settings(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    input: UpdateAppSettingsInput,
) -> Result<AppSettings, ApiError> {
    if !ALLOWED_DATE_FORMATS.contains(&input.date_format.as_str()) {
        return Err(ApiError::unprocessable_entity("Invalid date format"));
    }

    if !ALLOWED_DATETIME_FORMATS.contains(&input.datetime_format.as_str()) {
        return Err(ApiError::unprocessable_entity("Invalid datetime format"));
    }

    diesel::update(app_settings::table.filter(app_settings::id.eq(APP_SETTINGS_ID)))
        .set((
            app_settings::allow_user_registration.eq(input.allow_user_registration),
            app_settings::date_format.eq(input.date_format),
            app_settings::datetime_format.eq(input.datetime_format),
            app_settings::virus_scanning_enabled.eq(input.virus_scanning_enabled),
            app_settings::virus_scan_by_default.eq(input.virus_scan_by_default),
            app_settings::updated_at.eq(diesel::dsl::now),
        ))
        .returning(AppSettings::as_returning())
        .get_result(db)
        .await
        .api_context("Failed to update app settings")
}
