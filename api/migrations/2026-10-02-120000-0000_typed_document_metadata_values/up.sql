ALTER TABLE document_metadatas
ADD COLUMN string_value VARCHAR,
ADD COLUMN number_value NUMERIC,
ADD COLUMN date_value DATE;

DELETE FROM document_metadatas dm
USING metadata_types mt
WHERE mt.id = dm.metadata_type_id
  AND mt.data_type = 'date'
  AND NULLIF(BTRIM(dm.value), '') IS NULL;

UPDATE document_metadatas dm
SET string_value = dm.value
FROM metadata_types mt
WHERE mt.id = dm.metadata_type_id
  AND mt.data_type IN ('string', 'lookup');

DO $$
DECLARE
    date_metadata RECORD;
BEGIN
    FOR date_metadata IN
        SELECT dm.document_id, dm.metadata_type_id, dm.value
        FROM document_metadatas dm
        JOIN metadata_types mt ON mt.id = dm.metadata_type_id
        WHERE mt.data_type = 'date'
    LOOP
        IF BTRIM(date_metadata.value) !~ '^\d{4}-\d{2}-\d{2}$' THEN
            RAISE EXCEPTION 'Cannot migrate document_metadatas: document_id %, metadata_type_id % has date value % that is not in YYYY-MM-DD format',
                date_metadata.document_id,
                date_metadata.metadata_type_id,
                date_metadata.value;
        END IF;

        BEGIN
            PERFORM BTRIM(date_metadata.value)::date;
        EXCEPTION WHEN others THEN
            RAISE EXCEPTION 'Cannot migrate document_metadatas: document_id %, metadata_type_id % has invalid date value %',
                date_metadata.document_id,
                date_metadata.metadata_type_id,
                date_metadata.value;
        END;
    END LOOP;
END $$;

UPDATE document_metadatas dm
SET date_value = BTRIM(dm.value)::date
FROM metadata_types mt
WHERE mt.id = dm.metadata_type_id
  AND mt.data_type = 'date';

DO $$
DECLARE
    untyped_count BIGINT;
BEGIN
    SELECT COUNT(*)
    INTO untyped_count
    FROM document_metadatas dm
    JOIN metadata_types mt ON mt.id = dm.metadata_type_id
    WHERE mt.data_type NOT IN ('string', 'lookup', 'date');

    IF untyped_count > 0 THEN
        RAISE EXCEPTION 'Cannot migrate document_metadatas: % row(s) use unsupported metadata type data_type', untyped_count;
    END IF;
END $$;

DO $$
DECLARE
    untyped_count BIGINT;
BEGIN
    SELECT COUNT(*)
    INTO untyped_count
    FROM document_metadatas
    WHERE num_nonnulls(string_value, number_value, date_value) <> 1;

    IF untyped_count > 0 THEN
        RAISE EXCEPTION 'Cannot migrate document_metadatas: % row(s) did not map to exactly one typed value', untyped_count;
    END IF;
END $$;

DROP INDEX document_metadatas_metadata_type_id_value;
DROP INDEX document_metadatas_value;

ALTER TABLE document_metadatas
DROP COLUMN value,
ADD CONSTRAINT document_metadatas_exactly_one_value
CHECK (num_nonnulls(string_value, number_value, date_value) = 1);

CREATE INDEX document_metadatas_string_value ON document_metadatas(string_value);
CREATE INDEX document_metadatas_metadata_type_id_string_value ON document_metadatas(metadata_type_id, string_value);
