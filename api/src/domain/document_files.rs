use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::Serialize;

use crate::schema::{document_file_ocr_pages, document_file_pages, document_files};

/// Scan status for a document file that has no scan requirement.
pub const SCAN_STATUS_NOT_REQUIRED: &str = "not_required";
/// Scan status for a document file waiting for a scanner worker.
pub const SCAN_STATUS_PENDING: &str = "pending";
/// Scan status for a document file actively being scanned.
pub const SCAN_STATUS_SCANNING: &str = "scanning";
/// Scan status for a document file accepted by the scanner.
pub const SCAN_STATUS_CLEAN: &str = "clean";
/// Scan status for a document file rejected by the scanner.
pub const SCAN_STATUS_INFECTED: &str = "infected";
/// Scan status for a document file whose scan failed or timed out.
pub const SCAN_STATUS_ERROR: &str = "scan_error";

/// Returns true when a scan status permits reading the file's bytes.
///
/// Only `clean` and `not_required` files are available for download,
/// preview, and document processing. This is the single shared predicate;
/// callers must not reimplement the status list inline.
pub fn is_content_available_scan_status(scan_status: &str) -> bool {
    matches!(scan_status, "clean" | "not_required")
}

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
    pub scan_status: String,
    pub scan_requested: bool,
    pub scan_scanner: Option<String>,
    pub scan_scanner_version: Option<String>,
    pub scan_signature_version: Option<String>,
    pub scan_threat_name: Option<String>,
    pub scan_error: Option<String>,
    pub scan_started_at: Option<DateTime<Utc>>,
    pub scan_completed_at: Option<DateTime<Utc>>,
    pub scan_requested_by: Option<i64>,
    pub content_available: bool,
    pub created_at: DateTime<Utc>,
    pub created_by: i64,
    pub updated_at: DateTime<Utc>,
    pub updated_by: i64,
}

impl DocumentFile {
    /// Returns true when this file's bytes may be read for download,
    /// preview, or document processing.
    pub fn content_available(&self) -> bool {
        is_content_available_scan_status(&self.scan_status)
    }
}

#[derive(Debug, Serialize, PartialEq, Queryable, Selectable, utoipa::ToSchema)]
#[diesel(table_name = document_files)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct DocumentFileView {
    pub id: i64,
    pub document_id: i64,
    pub filename: String,
    pub content_type: Option<String>,
    pub size: i64,
    pub pages: i32,
    pub scan_status: String,
    pub scan_requested: bool,
    pub scan_scanner: Option<String>,
    pub scan_scanner_version: Option<String>,
    pub scan_signature_version: Option<String>,
    pub scan_threat_name: Option<String>,
    pub scan_started_at: Option<DateTime<Utc>>,
    pub scan_completed_at: Option<DateTime<Utc>>,
    /// Derived from `scan_status` by a GENERATED ALWAYS column: true only
    /// for `clean` and `not_required` files. Never written directly.
    pub content_available: bool,
    pub created_at: DateTime<Utc>,
    pub created_by: i64,
    pub updated_at: DateTime<Utc>,
    pub updated_by: i64,
}

#[derive(Debug, Serialize, PartialEq, Queryable, Selectable, utoipa::ToSchema)]
#[diesel(table_name = document_file_pages)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct DocumentFilePage {
    pub document_file_id: i64,
    pub page_number: i32,
    pub text_content: Option<String>,
}

#[derive(Debug, Serialize, PartialEq, Queryable, Selectable, utoipa::ToSchema)]
#[diesel(table_name = document_file_ocr_pages)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct DocumentFileOcrPage {
    pub document_file_id: i64,
    pub page_number: i32,
    pub ocr_content: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_clean_and_not_required_statuses_are_content_available() {
        assert!(is_content_available_scan_status(SCAN_STATUS_CLEAN));
        assert!(is_content_available_scan_status(SCAN_STATUS_NOT_REQUIRED));
        assert!(!is_content_available_scan_status(SCAN_STATUS_PENDING));
        assert!(!is_content_available_scan_status(SCAN_STATUS_SCANNING));
        assert!(!is_content_available_scan_status(SCAN_STATUS_INFECTED));
        assert!(!is_content_available_scan_status(SCAN_STATUS_ERROR));
        assert!(!is_content_available_scan_status("unexpected"));
    }
}
