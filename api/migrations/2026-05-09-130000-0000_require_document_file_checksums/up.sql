DROP INDEX IF EXISTS document_files_checksum_sha256_idx;

ALTER TABLE document_files
DROP CONSTRAINT document_files_checksum_sha256_valid;

ALTER TABLE document_files
ALTER COLUMN checksum_sha256 SET NOT NULL;

ALTER TABLE document_files
ADD CONSTRAINT document_files_checksum_sha256_valid
CHECK (checksum_sha256 ~ '^[0-9a-f]{64}$');

CREATE INDEX document_files_checksum_sha256_idx
ON document_files (checksum_sha256);
