DROP INDEX document_file_pages_text_ts_gin_idx;
ALTER TABLE document_file_pages DROP COLUMN text_ts;

DROP INDEX document_file_ocr_pages_ocr_ts_gin_idx;
ALTER TABLE document_file_ocr_pages DROP COLUMN ocr_ts;
