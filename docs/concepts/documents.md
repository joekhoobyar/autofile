# Documents

Documents are the primary records in Autofile. A document can have one or more uploaded files plus structured information such as title, metadata, tags, cabinets, and index values.

## Document Files

Uploaded files are stored in S3-compatible object storage. Autofile keeps the file metadata in PostgreSQL and stores the file bytes in the configured bucket.

## Processing

Autofile uses background jobs to process document files. Processing can include previews, extracted text, OCR content, thumbnails, and page data depending on the file type and available processing tools.

## Document View

The UI exposes document sections for properties, preview, metadata, indexes, files, extracted text, OCR content, classifier testing, and template testing.
