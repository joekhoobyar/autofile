ALTER TABLE document_file_pages ADD COLUMN text_ts tsvector
  GENERATED ALWAYS AS (to_tsvector('english', text_content)) STORED;
CREATE INDEX document_file_pages_text_ts_gin_idx ON document_file_pages USING GIN (text_ts);

ALTER TABLE document_file_ocr_pages ADD COLUMN ocr_ts tsvector
  GENERATED ALWAYS AS (to_tsvector('english', ocr_content)) STORED;
CREATE INDEX document_file_ocr_pages_ocr_ts_gin_idx ON document_file_ocr_pages USING GIN (ocr_ts);
