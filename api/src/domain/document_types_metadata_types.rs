use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::Serialize;

use crate::schema::document_types_metadata_types;
use crate::domain::document_types::DocumentType;
use crate::domain::metadata_types::MetadataType;

#[derive(Debug, Serialize, Identifiable, Associations, Queryable, Selectable)]
#[diesel(belongs_to(DocumentType))]
#[diesel(belongs_to(MetadataType))]
#[diesel(table_name = document_types_metadata_types)]
#[diesel(primary_key(document_type_id, metadata_type_id))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct DocumentTypeMetadataType {
    pub document_type_id: i64,
    pub metadata_type_id: i64,
    pub required: bool,
    pub updated_at: DateTime<Utc>,
}
