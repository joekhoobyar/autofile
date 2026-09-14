# Background Jobs

Autofile uses Valkey-backed background jobs for document processing work.

## Valkey

The API connects to Valkey through `REDIS_URL`. The Docker Compose stack provides a Valkey service automatically.

## Job Queues

The API starts in-process workers for fast, medium, and slow jobs. These queues let Autofile process document-related work without blocking normal API requests.

## Processing Work

Background jobs handle work such as preview generation, text extraction, OCR, thumbnails, and document page processing.

## Virus Scan Job

When an upload requests virus scanning, the API stores the file and enqueues `ScanDocumentFile { document_file_id }` on the medium queue instead of the normal post-upload processing jobs. The worker marks the file `scanning`, streams the stored object to the configured `clamd` endpoint, and records the verdict:

- `clean` enqueues the normal page-processing and thumbnail jobs for the file.
- `infected` blocks the file without enqueueing further processing.
- Scan failures mark the file `scan_error` under the fail-closed policy (or `not_required` under the fail-open policy, which then enqueues normal processing).

If post-upload enqueueing fails after a clean scan, the file is reset to `scan_error` so the next retry can recover instead of leaving it stuck. Rescans requested through `POST /api/v1/documents/{document_id}/files/{id}/rescan` reuse the same job. ClamAV signature databases are large and take time to initialize, so avoid sending scan jobs to ClamAV before it is ready.

## Operations

If document processing appears stuck, check:

- The API logs.
- Valkey connectivity.
- S3 bucket access.
