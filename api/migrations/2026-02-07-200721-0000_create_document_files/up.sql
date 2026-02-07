CREATE TABLE document_files (
    id BIGSERIAL PRIMARY KEY,
    document_id BIGINT NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    s3_prefix VARCHAR(36) NOT NULL,
    filename VARCHAR(512) NOT NULL,
    content_type VARCHAR(255),
    size BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by BIGINT NOT NULL REFERENCES users(id),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_by BIGINT NOT NULL REFERENCES users(id)
);
