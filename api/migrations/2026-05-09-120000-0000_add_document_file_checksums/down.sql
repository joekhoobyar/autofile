DROP INDEX document_files_checksum_sha256_idx;

ALTER TABLE document_files
DROP CONSTRAINT document_files_checksum_sha256_valid;

ALTER TABLE document_files
DROP COLUMN checksum_sha256;
