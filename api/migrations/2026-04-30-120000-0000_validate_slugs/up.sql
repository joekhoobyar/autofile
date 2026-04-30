ALTER TABLE cabinets
    ADD CONSTRAINT cabinets_slug_format
    CHECK (slug ~ '^[a-z0-9_-]+$');

ALTER TABLE document_types
    ADD CONSTRAINT document_types_slug_format
    CHECK (slug ~ '^[a-z0-9_-]+$');

ALTER TABLE metadata_types
    ADD CONSTRAINT metadata_types_slug_format
    CHECK (slug ~ '^[a-z0-9_-]+$');

ALTER TABLE tags
    ADD CONSTRAINT tags_slug_format
    CHECK (slug ~ '^[a-z0-9_-]+$');

ALTER TABLE document_indexes
    ADD CONSTRAINT document_indexes_slug_format
    CHECK (slug ~ '^[a-z0-9_-]+$');
