ALTER TABLE app_settings
ADD COLUMN date_format TEXT NOT NULL DEFAULT 'yyyy-MM-dd',
ADD COLUMN datetime_format TEXT NOT NULL DEFAULT 'MM/dd/yyyy HH:mm';
