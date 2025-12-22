-- Your SQL goes here


CREATE TABLE "document_types"(
	"id" INT8 NOT NULL PRIMARY KEY,
	"slug" VARCHAR NOT NULL,
	"name" VARCHAR NOT NULL,
	"description" VARCHAR,
	"created_at" TIMESTAMPTZ NOT NULL,
	"updated_at" TIMESTAMPTZ NOT NULL
);

