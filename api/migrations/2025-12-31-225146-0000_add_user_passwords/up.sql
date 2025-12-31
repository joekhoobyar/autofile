-- Your SQL goes here
alter table users
  add column password_hash text not null,
  add column password_changed_at timestamptz not null;
