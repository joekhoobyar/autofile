-- Your SQL goes here
alter table documents
  add column created_by bigserial not null,
  add column updated_by bigserial not null;
