CREATE TABLE IF NOT EXISTS "document_indexes" (
	"id" BIGSERIAL PRIMARY KEY,
	"slug" VARCHAR NOT NULL UNIQUE,
	"name" VARCHAR NOT NULL,
	"description" VARCHAR NULL,
	"enabled" boolean NOT NULL DEFAULT true,
	"created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "created_by" INT8 NOT NULL REFERENCES users(id),
	"updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_by" INT8 NOT NULL REFERENCES users(id)
);

CREATE TABLE IF NOT EXISTS "document_index_templates" (
	"id" BIGSERIAL PRIMARY KEY,
	"template" VARCHAR NOT NULL,
	"is_leaf" boolean NOT NULL,
	"enabled" boolean NOT NULL DEFAULT true,
    "document_index_id" INT8 NOT NULL REFERENCES document_indexes(id),
    "parent_id" INT8 NULL REFERENCES document_index_templates(id),
	"created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "created_by" INT8 NOT NULL REFERENCES users(id),
	"updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_by" INT8 NOT NULL REFERENCES users(id)
);
CREATE INDEX document_index_templates_parent_id
    ON document_index_templates (parent_id);
CREATE INDEX document_index_templates_document_index_id
    ON document_index_templates (document_index_id);

CREATE TABLE IF NOT EXISTS "document_index_values" (
	"id" BIGSERIAL PRIMARY KEY,
	"value" VARCHAR NOT NULL,
    "document_index_template_id" INT8 NOT NULL REFERENCES document_index_templates(id),
    "parent_id" INT8 NULL REFERENCES document_index_values(id),
    CONSTRAINT "document_index_values_template_value"
        UNIQUE (document_index_template_id, value)
);
CREATE INDEX document_index_values_parent_id
    ON document_index_values (parent_id);
CREATE INDEX document_index_values_value
    ON document_index_values (value);

CREATE TABLE IF NOT EXISTS "document_index_documents" (
    "document_index_value_id" INT8 NULL REFERENCES document_index_values(id),
    "document_id" INT8 NOT NULL REFERENCES documents(id),
    PRIMARY KEY(document_index_value_id, document_id)
);
CREATE INDEX document_index_documents_document_id
    ON document_index_documents (document_id);
