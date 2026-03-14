-- Your SQL goes here

CREATE TABLE cabinet_documents (
    cabinet_id INT8 NOT NULL REFERENCES cabinets(id),
    document_id INT8 NOT NULL REFERENCES documents(id),
    PRIMARY KEY(cabinet_id, document_id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by BIGINT NOT NULL REFERENCES users(id),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_by BIGINT NOT NULL REFERENCES users(id)
);

-- Your SQL goes here
