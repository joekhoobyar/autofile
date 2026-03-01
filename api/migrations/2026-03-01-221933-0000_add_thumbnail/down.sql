-- This file should undo anything in `up.sql`
alter table documents
  drop column s3_thumbnail;
