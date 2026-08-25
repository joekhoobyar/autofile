# Documents

Documents are the primary records in Autofile. A document has one or more uploaded files plus structured information such as title, metadata, tags, cabinets, and index values.

Each document has one [Document Type](document-types.md), which determines the fields available on its [Metadata](document-metadata.md) page.

## Document Files

Uploaded files are stored in S3-compatible object storage. Autofile keeps the file metadata in PostgreSQL and stores the file bytes in the configured bucket.

### Supported File Types

Autofile can process these uploaded file types:

| Type | Extensions | Notes |
| --- | --- | --- |
| PDF | `.pdf` | Extracts pages, text, previews, and OCR content. |
| Images | `.jpg`, `.jpeg`, `.jfif`, `.png`, `.tif`, `.tiff`, `.svg`, `.gif`, `.webp`, `.bmp`, `.heic`, `.heif`, `.avif`, `.ico` | Any upload with an `image/*` content type is treated as an image. Image documents are processed as a single page with OCR. |
| Plain text | `.txt`, `.text` | Converted to PDF for page previews and text extraction. |
| Markdown | `.md`, `.markdown` | Also supports `text/markdown` and `text/x-markdown` uploads. |
| CSV | `.csv` | Also supports CSV files uploaded as `text/plain`. |
| TSV | `.tsv` | Also supports TSV files uploaded as `text/plain`. |
| Word processing documents | `.doc`, `.docx`, `.odt` | Converted to PDF before processing. |
| Spreadsheets | `.xls`, `.xlsx`, `.ods` | Converted to PDF before processing. |
| Presentations | `.ppt`, `.pptx`, `.odp` | Converted to PDF before processing. |
| HTML | `.html`, `.htm`, `.xhtml` | Converted to PDF before processing. |

If an upload is missing a content type or arrives as a generic `application/octet-stream` or `text/plain`, Autofile infers a more specific type from these extensions when possible. Unsupported content types are stored, but Autofile cannot process their pages.

## Processing

Autofile uses background jobs to process document files. Processing includes previews, extracted text, OCR content, thumbnails, and page data depending on the file type.

## Document View

The UI exposes document sections for properties, preview, metadata, indexes, files, extracted text, OCR content, classifier testing, and template testing.

Use **Properties** to change the title or Document Type. Changing the Document Type removes metadata fields that are not associated with the new type, so review the [Document Types guide](document-types.md#assign-a-document-type) before making that change.
