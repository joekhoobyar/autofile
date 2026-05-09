ALTER TABLE document_files
ADD COLUMN checksum_sha256 VARCHAR(64);

ALTER TABLE document_files
ADD CONSTRAINT document_files_checksum_sha256_valid
CHECK (
    checksum_sha256 IS NULL
    OR checksum_sha256 ~ '^[0-9a-f]{64}$'
);

CREATE INDEX document_files_checksum_sha256_idx
ON document_files (checksum_sha256)
WHERE checksum_sha256 IS NOT NULL;
