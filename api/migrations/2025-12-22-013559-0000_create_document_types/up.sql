-- Your SQL goes here


CREATE TABLE "document_types"(
	"id" BIGSERIAL PRIMARY KEY,
	"slug" VARCHAR NOT NULL UNIQUE,
	"name" VARCHAR NOT NULL,
	"description" VARCHAR,
	"created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
	"updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

