ALTER TABLE document_metadatas
ADD COLUMN value VARCHAR;

UPDATE document_metadatas
SET value = COALESCE(
    string_value,
    number_value::TEXT,
    to_char(date_value, 'YYYY-MM-DD')
);

ALTER TABLE document_metadatas
ALTER COLUMN value SET NOT NULL,
DROP CONSTRAINT document_metadatas_exactly_one_value;

DROP INDEX document_metadatas_metadata_type_id_string_value;
DROP INDEX document_metadatas_string_value;

ALTER TABLE document_metadatas
DROP COLUMN string_value,
DROP COLUMN number_value,
DROP COLUMN date_value;

CREATE INDEX document_metadatas_value ON document_metadatas(value);
CREATE INDEX document_metadatas_metadata_type_id_value ON document_metadatas(metadata_type_id, value);
