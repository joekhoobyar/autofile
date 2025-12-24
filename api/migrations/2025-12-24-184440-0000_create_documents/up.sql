-- Your SQL goes here

CREATE TABLE "documents"(
	"id" BIGSERIAL PRIMARY KEY,
	"title" VARCHAR NOT NULL,
    "document_type_id" INT8 NOT NULL REFERENCES document_types(id),
	"created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
	"updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

