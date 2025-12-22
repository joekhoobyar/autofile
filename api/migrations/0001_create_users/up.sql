create table users (
  id bigserial primary key,
  username varchar not null unique,
  email varchar not null unique,
  display_name varchar not null,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);
