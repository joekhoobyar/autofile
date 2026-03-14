-- Your SQL goes here

CREATE TABLE document_metadatas (
    document_id INT8 NOT NULL REFERENCES documents(id),
    metadata_type_id INT8 NOT NULL REFERENCES metadata_types(id),
    PRIMARY KEY(document_id, metadata_type_id),
    value VARCHAR NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by BIGINT NOT NULL REFERENCES users(id),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_by BIGINT NOT NULL REFERENCES users(id)
);

