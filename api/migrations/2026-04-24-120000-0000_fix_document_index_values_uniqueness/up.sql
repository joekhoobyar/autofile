ALTER TABLE document_index_values
    DROP CONSTRAINT IF EXISTS document_index_values_template_value;

ALTER TABLE document_index_values
    ADD CONSTRAINT document_index_values_template_parent_value
    UNIQUE NULLS NOT DISTINCT (document_index_template_id, parent_id, value);
