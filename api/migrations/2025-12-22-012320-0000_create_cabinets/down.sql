-- This file should undo anything in `up.sql`
ALTER TABLE "users" DROP COLUMN "email";

DROP TABLE IF EXISTS "cabinets";
