-- Your SQL goes here
ALTER TABLE cabinets
ADD COLUMN parent_id bigint references cabinets(id) on delete set null;
