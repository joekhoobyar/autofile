-- Your SQL goes here
ALTER TABLE "users" ADD COLUMN "email" VARCHAR NOT NULL;

CREATE TABLE "cabinets"(
	"id" INT8 NOT NULL PRIMARY KEY,
	"slug" VARCHAR NOT NULL,
	"name" VARCHAR NOT NULL,
	"description" VARCHAR,
	"created_at" TIMESTAMPTZ NOT NULL,
	"updated_at" TIMESTAMPTZ NOT NULL
);

