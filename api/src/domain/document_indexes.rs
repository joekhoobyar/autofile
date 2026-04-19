use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::Serialize;

use crate::schema::{document_index_templates, document_index_values, document_indexes};

#[derive(Debug, Serialize, Identifiable, PartialEq, Queryable, Selectable)]
#[diesel(table_name = document_indexes)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct DocumentIndex {
    pub id: i64,
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub created_by: i64,
    pub created_at: DateTime<Utc>,
    pub updated_by: i64,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct DocumentIndexView {
    pub id: i64,
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub document_count: i64,
    pub created_by: i64,
    pub created_at: DateTime<Utc>,
    pub updated_by: i64,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Identifiable, PartialEq, Queryable, Selectable)]
#[diesel(table_name = document_index_templates)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct DocumentIndexTemplate {
    pub id: i64,
    pub template: String,
    pub is_leaf: bool,
    pub enabled: bool,
    pub document_index_id: i64,
    pub parent_id: Option<i64>,
    pub created_by: i64,
    pub created_at: DateTime<Utc>,
    pub updated_by: i64,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Identifiable, PartialEq, Queryable, Selectable)]
#[diesel(table_name = document_index_values)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct DocumentIndexValue {
    pub id: i64,
    pub value: String,
    pub document_index_id: i64,
    pub document_index_template_id: i64,
    pub parent_id: Option<i64>,
    pub is_leaf: bool,
}
