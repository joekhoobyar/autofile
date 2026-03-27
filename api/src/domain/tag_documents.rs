use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::Serialize;

use crate::schema::tag_documents;

#[derive(Debug, Serialize, Identifiable, PartialEq, Queryable, Selectable)]
#[diesel(belongs_to(Tag))]
#[diesel(belongs_to(Document))]
#[diesel(table_name = tag_documents)]
#[diesel(primary_key(tag_id, document_id))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct TagDocument {
    pub tag_id: i64,
    pub document_id: i64,
    pub updated_at: DateTime<Utc>,
    pub updated_by: i64,
}
