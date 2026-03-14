use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::Serialize;

use crate::schema::document_metadatas;

#[derive(Debug, Serialize, Identifiable, PartialEq, Queryable, Selectable)]
#[diesel(belongs_to(Document))]
#[diesel(belongs_to(MetadataType))]
#[diesel(table_name = document_metadatas)]
#[diesel(primary_key(document_id, metadata_type_id))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct DocumentMetadata {
    pub document_id: i64,
    pub metadata_type_id: i64,
    pub value: String,
    pub created_at: DateTime<Utc>,
    pub created_by: i64,
    pub updated_at: DateTime<Utc>,
    pub updated_by: i64,
}
