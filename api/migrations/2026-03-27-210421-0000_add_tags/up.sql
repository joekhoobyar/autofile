
CREATE TABLE tags (
	"id" BIGSERIAL PRIMARY KEY,
	"slug" VARCHAR NOT NULL UNIQUE,
	"name" VARCHAR NOT NULL,
	"color" CHAR(6) NOT NULL,
	"created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "created_by" INT8 NOT NULL REFERENCES users(id),
	"updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_by" INT8 NOT NULL REFERENCES users(id)
);

CREATE TABLE tag_documents (
    tag_id INT8 NOT NULL REFERENCES tags(id),
    document_id INT8 NOT NULL REFERENCES documents(id),
    PRIMARY KEY(tag_id, document_id),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_by BIGINT NOT NULL REFERENCES users(id)
);
