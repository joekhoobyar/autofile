use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::Serialize;

use crate::schema::{document_file_ocr_pages, document_file_pages, document_files};

#[derive(Debug, Serialize, Identifiable, PartialEq, Queryable, Selectable)]
#[diesel(table_name = document_files)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct DocumentFile {
    pub id: i64,
    pub document_id: i64,
    pub s3_prefix: String,
    pub filename: String,
    pub content_type: Option<String>,
    pub size: i64,
    pub checksum_sha256: String,
    pub pages: i32,
    pub created_at: DateTime<Utc>,
    pub created_by: i64,
    pub updated_at: DateTime<Utc>,
    pub updated_by: i64,
}

#[derive(Debug, Serialize, PartialEq, Queryable, Selectable)]
#[diesel(table_name = document_files)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct DocumentFileView {
    pub id: i64,
    pub document_id: i64,
    pub filename: String,
    pub content_type: Option<String>,
    pub size: i64,
    pub pages: i32,
    pub created_at: DateTime<Utc>,
    pub created_by: i64,
    pub updated_at: DateTime<Utc>,
    pub updated_by: i64,
}

#[derive(Debug, Serialize, PartialEq, Queryable, Selectable)]
#[diesel(table_name = document_file_pages)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct DocumentFilePage {
    pub document_file_id: i64,
    pub page_number: i32,
    pub text_content: Option<String>,
}

#[derive(Debug, Serialize, PartialEq, Queryable, Selectable)]
#[diesel(table_name = document_file_ocr_pages)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct DocumentFileOcrPage {
    pub document_file_id: i64,
    pub page_number: i32,
    pub ocr_content: Option<String>,
}
