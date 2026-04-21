alter table users
  add column role varchar;

update users
set role = 'admin'
where role is null;

alter table users
  alter column role set not null,
  alter column role set default 'user';

alter table users
  add constraint users_role_valid
  check (role in ('admin', 'user'));
