DROP INDEX document_files_scan_status_idx;

-- Drop the generated column before scan_status, which it depends on.
ALTER TABLE document_files
DROP COLUMN content_available;

ALTER TABLE document_files
DROP CONSTRAINT document_files_scan_requested_consistent;

ALTER TABLE document_files
DROP CONSTRAINT document_files_scan_status_valid;

ALTER TABLE document_files
DROP COLUMN scan_requested_by,
DROP COLUMN scan_completed_at,
DROP COLUMN scan_started_at,
DROP COLUMN scan_error,
DROP COLUMN scan_threat_name,
DROP COLUMN scan_signature_version,
DROP COLUMN scan_scanner_version,
DROP COLUMN scan_scanner,
DROP COLUMN scan_requested,
DROP COLUMN scan_status;
