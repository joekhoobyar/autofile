-- Your SQL goes here

CREATE TABLE "metadata_types"(
	"id" INT8 NOT NULL PRIMARY KEY,
	"slug" VARCHAR NOT NULL,
	"name" VARCHAR NOT NULL,
	"data_type" VARCHAR NOT NULL,
	"description" VARCHAR,
	"created_at" TIMESTAMPTZ NOT NULL,
	"updated_at" TIMESTAMPTZ NOT NULL
);
