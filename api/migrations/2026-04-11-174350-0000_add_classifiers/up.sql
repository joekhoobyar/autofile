CREATE TABLE IF NOT EXISTS "classifier_blocks" (
	"id" BIGSERIAL PRIMARY KEY,
	"name" VARCHAR NOT NULL UNIQUE,
	"description" VARCHAR NULL,
	"enabled" boolean NOT NULL DEFAULT true,
	"order" INT NOT NULL UNIQUE,
	"rules" JSONB NOT NULL,
	"created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "created_by" INT8 NOT NULL REFERENCES users(id),
	"updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_by" INT8 NOT NULL REFERENCES users(id)
);
