use bigdecimal::BigDecimal;
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
    pub created_at: DateTime<Utc>,
    pub created_by: i64,
    pub updated_at: DateTime<Utc>,
    pub updated_by: i64,
    /// Stored string or lookup metadata value. Exactly one typed value field is non-null for stored rows.
    #[schema(example = "Acme Corporation")]
    pub string_value: Option<String>,
    /// Stored numeric metadata value. This field is reserved; number metadata types are not supported yet.
    #[schema(value_type = Option<f64>, example = 123.45)]
    pub number_value: Option<BigDecimal>,
    /// Stored date metadata value. API dates use YYYY-MM-DD regardless of the configured frontend display format.
    #[schema(example = "2026-08-25")]
    pub date_value: Option<chrono::NaiveDate>,
}
