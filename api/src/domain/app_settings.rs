use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::Serialize;

use crate::schema::app_settings;

#[derive(Debug, Serialize, Identifiable, PartialEq, Queryable, Selectable, utoipa::ToSchema)]
#[diesel(table_name = app_settings)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct AppSettings {
    pub id: i64,
    pub allow_user_registration: bool,
    /// Frontend-only date display and date-picker format. Date metadata API values remain YYYY-MM-DD.
    #[schema(example = "yyyy-MM-dd")]
    pub date_format: String,
    /// Frontend-only date/time display format for timestamps. API timestamps remain serialized as timestamp strings.
    #[schema(example = "MM/dd/yyyy HH:mm")]
    pub datetime_format: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
