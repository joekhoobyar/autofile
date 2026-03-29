ALTER TABLE document_index_values
    ADD COLUMN is_leaf boolean NOT NULL DEFAULT false;

UPDATE document_index_values v
SET is_leaf = t.is_leaf
FROM document_index_templates t
WHERE v.document_index_template_id = t.id;
