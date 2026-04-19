INSERT INTO users (
    username,
    email,
    display_name,
    password_hash,
    password_changed_at
)
VALUES (
    'system',
    'system',
    'System',
    'NEVER',
    NOW()
)
ON CONFLICT (username) DO NOTHING;

WITH seed_user AS ( SELECT id FROM users WHERE username = 'system' LIMIT 1 )
INSERT INTO cabinets (slug, name, description, created_by, updated_by)
SELECT 'inbox', 'Inbox', 'Default intake cabinet', id, id
FROM seed_user
ON CONFLICT (slug) DO NOTHING;

WITH seed_user AS ( SELECT id FROM users WHERE username = 'system' LIMIT 1 )
INSERT INTO document_types (slug, name, description, created_by, updated_by)
SELECT 'unspecified', 'Unspecified', 'Unspecified type of document', id, id
FROM seed_user
ON CONFLICT (slug) DO NOTHING;

WITH seed_user AS ( SELECT id FROM users WHERE username = 'system' LIMIT 1 )
INSERT INTO metadata_types (slug, name, data_type, description, options, created_by, updated_by)
SELECT 'correspondent', 'Correspondent', 'string', 'The corresponding party', NULL, id, id
FROM seed_user
ON CONFLICT (slug) DO NOTHING;

WITH seed_user AS ( SELECT id FROM users WHERE username = 'system' LIMIT 1 )
INSERT INTO metadata_types (slug, name, data_type, description, options, created_by, updated_by)
SELECT 'issue_date', 'Issue Date', 'date', 'The date when the document was issued', NULL, id, id
FROM seed_user
ON CONFLICT (slug) DO NOTHING;

INSERT INTO document_types_metadata_types (document_type_id, metadata_type_id, required)
SELECT dt.id, mt.id, true
FROM document_types dt
JOIN metadata_types mt ON mt.slug = 'correspondent'
WHERE dt.slug = 'unspecified'
ON CONFLICT (document_type_id, metadata_type_id) DO NOTHING;

INSERT INTO document_types_metadata_types (document_type_id, metadata_type_id, required)
SELECT dt.id, mt.id, true
FROM document_types dt
JOIN metadata_types mt ON mt.slug = 'issue_date'
WHERE dt.slug = 'unspecified'
ON CONFLICT (document_type_id, metadata_type_id) DO NOTHING;
