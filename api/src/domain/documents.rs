use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::Serialize;

use crate::schema::documents;

#[derive(Debug, Serialize, Identifiable, PartialEq, Queryable, Selectable)]
#[diesel(table_name = documents)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Document {
    pub id: i64,
    pub title: String,
    pub document_type_id: i64,
    pub s3_thumbnail: Option<String>,
    pub created_by: i64,
    pub created_at: DateTime<Utc>,
    pub updated_by: i64,
    pub updated_at: DateTime<Utc>,
}
