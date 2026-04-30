ALTER TABLE document_indexes
    DROP CONSTRAINT IF EXISTS document_indexes_slug_format;

ALTER TABLE tags
    DROP CONSTRAINT IF EXISTS tags_slug_format;

ALTER TABLE metadata_types
    DROP CONSTRAINT IF EXISTS metadata_types_slug_format;

ALTER TABLE document_types
    DROP CONSTRAINT IF EXISTS document_types_slug_format;

ALTER TABLE cabinets
    DROP CONSTRAINT IF EXISTS cabinets_slug_format;
