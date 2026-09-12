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
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
