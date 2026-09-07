ALTER TABLE users
  ADD COLUMN force_password_change boolean NOT NULL DEFAULT false;
