use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::Serialize;

use crate::schema::document_metadatas;

#[derive(Debug, Serialize, Identifiable, PartialEq, Queryable, Selectable, utoipa::ToSchema)]
#[diesel(belongs_to(Document))]
#[diesel(belongs_to(MetadataType))]
#[diesel(table_name = document_metadatas)]
#[diesel(primary_key(document_id, metadata_type_id))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct DocumentMetadata {
    pub document_id: i64,
    pub metadata_type_id: i64,
    /// Stored metadata value. For Date metadata, clients must send and receive values in YYYY-MM-DD format regardless of the configured frontend display format.
    #[schema(example = "2026-08-25")]
    pub value: String,
    pub created_at: DateTime<Utc>,
    pub created_by: i64,
    pub updated_at: DateTime<Utc>,
    pub updated_by: i64,
}
