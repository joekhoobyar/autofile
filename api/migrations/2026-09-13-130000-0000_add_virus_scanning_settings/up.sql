ALTER TABLE app_settings
ADD COLUMN virus_scanning_enabled BOOLEAN NOT NULL DEFAULT false,
ADD COLUMN virus_scan_by_default BOOLEAN NOT NULL DEFAULT true;
