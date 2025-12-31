-- This file should undo anything in `up.sql`
alter table users
  drop column password_hash,
  drop column password_changed_at;
