ALTER TABLE cabinet_documents
    ADD COLUMN created_at TIMESTAMPTZ DEFAULT NOW(),
    ADD COLUMN created_by BIGINT REFERENCES users(id);
UPDATE cabinet_documents
SET created_at = updated_at,
    created_by = updated_by;
ALTER TABLE cabinet_documents
    ALTER COLUMN created_at SET NOT NULL,
    ALTER COLUMN created_by SET NOT NULL;

ALTER TABLE document_types_metadata_types
    DROP COLUMN updated_at;

ALTER TABLE cabinets
    DROP COLUMN created_by,
    DROP COLUMN updated_by;

ALTER TABLE document_types
    DROP COLUMN created_by,
    DROP COLUMN updated_by;

ALTER TABLE metadata_types
    DROP COLUMN created_by,
    DROP COLUMN updated_by;
