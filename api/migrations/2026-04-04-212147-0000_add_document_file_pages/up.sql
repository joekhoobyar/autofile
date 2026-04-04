-- Your SQL goes here
CREATE TABLE IF NOT EXISTS "document_file_pages" (
	"document_file_id" INT8 NOT NULL REFERENCES document_files(id),
	"page_number" INT NOT NULL,
    "text_content" TEXT,
    PRIMARY KEY(document_file_id, page_number)
);

ALTER TABLE document_files
    ADD COLUMN pages INT NOT NULL DEFAULT 0;
