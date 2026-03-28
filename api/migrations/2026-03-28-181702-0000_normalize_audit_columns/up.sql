ALTER TABLE cabinet_documents
    DROP COLUMN created_at,
    DROP COLUMN created_by;

ALTER TABLE document_types_metadata_types
    ADD COLUMN updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW();

ALTER TABLE cabinets
    ADD COLUMN created_by BIGINT REFERENCES users(id),
    ADD COLUMN updated_by BIGINT REFERENCES users(id);
UPDATE cabinets
SET created_by = u.id,
    updated_by = u.id
FROM (SELECT MIN(id) id FROM USERS) u;
ALTER TABLE cabinets
    ALTER COLUMN created_by SET NOT NULL,
    ALTER COLUMN updated_by SET NOT NULL;

ALTER TABLE document_types
    ADD COLUMN created_by BIGINT REFERENCES users(id),
    ADD COLUMN updated_by BIGINT REFERENCES users(id);
UPDATE document_types
SET created_by = u.id,
    updated_by = u.id
FROM (SELECT MIN(id) id FROM USERS) u;
ALTER TABLE document_types
    ALTER COLUMN created_by SET NOT NULL,
    ALTER COLUMN updated_by SET NOT NULL;

ALTER TABLE metadata_types
    ADD COLUMN created_by BIGINT REFERENCES users(id),
    ADD COLUMN updated_by BIGINT REFERENCES users(id);
UPDATE metadata_types
SET created_by = u.id,
    updated_by = u.id
FROM (SELECT MIN(id) id FROM USERS) u;
ALTER TABLE metadata_types
    ALTER COLUMN created_by SET NOT NULL,
    ALTER COLUMN updated_by SET NOT NULL;
