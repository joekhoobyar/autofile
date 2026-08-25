# Documents

Documents are the primary records in Autofile. A document can have uploaded files plus structured information such as title, metadata, tags, cabinets, and index values.

Each document has one [Document Type](document-types.md), which determines the fields available on its [Metadata](document-metadata.md) page.

## Document Files

Uploaded files are stored in S3-compatible object storage. Autofile keeps the file metadata in PostgreSQL and stores the file bytes in the configured bucket.

## Processing

Autofile uses background jobs to process document files. Processing can include previews, extracted text, OCR content, thumbnails, and page data depending on the file type and available processing tools.

## Document View

The UI exposes document sections for properties, preview, metadata, indexes, files, extracted text, OCR content, classifier testing, and template testing.

Use **Properties** to change the title or Document Type. Changing the Document Type removes metadata fields that are not associated with the new type, so review the [Document Types guide](document-types.md#assign-a-document-type) before making that change.
