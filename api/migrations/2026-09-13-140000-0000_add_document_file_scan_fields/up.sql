ALTER TABLE document_files
ADD COLUMN scan_status VARCHAR(32) NOT NULL DEFAULT 'not_required',
ADD COLUMN scan_requested BOOLEAN NOT NULL DEFAULT false,
ADD COLUMN scan_scanner VARCHAR(64),
ADD COLUMN scan_scanner_version VARCHAR(128),
ADD COLUMN scan_signature_version VARCHAR(128),
ADD COLUMN scan_threat_name VARCHAR(512),
ADD COLUMN scan_error TEXT,
ADD COLUMN scan_started_at TIMESTAMPTZ,
ADD COLUMN scan_completed_at TIMESTAMPTZ,
ADD COLUMN scan_requested_by BIGINT REFERENCES users(id) ON DELETE SET NULL;

ALTER TABLE document_files
ADD CONSTRAINT document_files_scan_status_valid
CHECK (
    scan_status IN ('not_required', 'pending', 'scanning', 'clean', 'infected', 'scan_error')
);

ALTER TABLE document_files
ADD CONSTRAINT document_files_scan_requested_consistent
CHECK (
    (scan_requested = FALSE AND scan_status = 'not_required')
    OR (scan_requested = TRUE AND scan_status <> 'not_required')
);

-- Derived availability flag: true only for `clean` and `not_required`
-- files. GENERATED ALWAYS keeps `scan_status` as the single stored
-- authority; the flag can never drift from it.
ALTER TABLE document_files
ADD COLUMN content_available BOOLEAN
GENERATED ALWAYS AS (scan_status IN ('clean', 'not_required')) STORED;

CREATE INDEX document_files_scan_status_idx
ON document_files (scan_status)
WHERE scan_status <> 'not_required';
