ALTER TABLE document_index_values
    DROP CONSTRAINT IF EXISTS document_index_values_template_parent_value;

ALTER TABLE document_index_values
    ADD CONSTRAINT document_index_values_template_value
    UNIQUE (document_index_template_id, value);
