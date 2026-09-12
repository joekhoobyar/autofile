use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::schema::documents;

#[derive(Debug, Serialize, Identifiable, PartialEq, Queryable, Selectable, utoipa::ToSchema)]
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

#[derive(Debug, Serialize, PartialEq, utoipa::ToSchema)]
pub struct DocumentView {
    pub id: i64,
    pub title: String,
    pub document_type_id: i64,
    pub pages: i32,
    /// Stored metadata values keyed by Metadata Type slug.
    #[schema(schema_with = metadata_map_schema)]
    pub metadata: HashMap<String, String>,
    pub cabinet_ids: Vec<i64>,
    pub tag_ids: Vec<i64>,
    pub created_by: i64,
    pub created_at: DateTime<Utc>,
    pub updated_by: i64,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct TemplateDocumentView {
    pub id: i64,
    pub title: String,
    pub document_type_id: i64,
    pub document_type: String,
    pub metadata: HashMap<String, String>,
    pub cabinet_ids: Vec<i64>,
    pub tag_ids: Vec<i64>,
    pub cabinets: HashSet<String>,
    pub tags: HashSet<String>,
    pub created_by: i64,
    pub created_at: DateTime<Utc>,
    pub updated_by: i64,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, AsChangeset, utoipa::ToSchema)]
#[diesel(table_name = documents)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct DocumentChangeset {
    pub title: Option<String>,
    pub document_type_id: Option<i64>,
}

/// Free-form string map schema for document metadata.
///
/// utoipa renders `HashMap` fields with a redundant `propertyNames` keyword
/// that Swagger UI displays as a confusing extra child; this keeps the
/// accurate `additionalProperties: string` shape without the noise.
fn metadata_map_schema() -> utoipa::openapi::Object {
    let string_schema =
        utoipa::openapi::ObjectBuilder::new().schema_type(utoipa::openapi::schema::Type::String);
    utoipa::openapi::ObjectBuilder::new()
        .description(Some("Stored metadata values keyed by Metadata Type slug"))
        .additional_properties(Some(string_schema))
        .build()
}
