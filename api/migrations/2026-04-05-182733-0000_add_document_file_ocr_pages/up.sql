CREATE TABLE IF NOT EXISTS "document_file_ocr_pages" (
	"document_file_id" INT8 NOT NULL REFERENCES document_files(id),
	"page_number" INT NOT NULL,
    "ocr_content" TEXT,
    PRIMARY KEY(document_file_id, page_number)
);
