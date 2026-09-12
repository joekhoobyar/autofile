use bb8::PooledConnection;
use diesel::prelude::*;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::{AsyncPgConnection, RunQueryDsl};

use crate::domain::app_settings::AppSettings;
use crate::schema::app_settings;
use crate::shared::errors::{ApiError, ApiErrorContext};

const APP_SETTINGS_ID: i64 = 1;

#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct UpdateAppSettingsInput {
    pub allow_user_registration: bool,
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
    diesel::update(app_settings::table.filter(app_settings::id.eq(APP_SETTINGS_ID)))
        .set((
            app_settings::allow_user_registration.eq(input.allow_user_registration),
            app_settings::updated_at.eq(diesel::dsl::now),
        ))
        .returning(AppSettings::as_returning())
        .get_result(db)
        .await
        .api_context("Failed to update app settings")
}
