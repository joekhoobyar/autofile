use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::Serialize;

use crate::schema::document_types;

#[derive(Debug, Serialize, Identifiable, PartialEq, Queryable, Selectable)]
#[diesel(table_name = document_types)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct DocumentType {
    pub id: i64,
    pub slug: String,
    pub name: String,
    pub created_by: i64,
    pub created_at: DateTime<Utc>,
    pub updated_by: i64,
    pub updated_at: DateTime<Utc>,
    pub description: Option<String>
}
