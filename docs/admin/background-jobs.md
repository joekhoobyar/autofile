# Background Jobs

Autofile uses Valkey-backed background jobs for document processing work.

## Valkey

The API connects to Valkey through `REDIS_URL`. The Docker Compose stack provides a Valkey service automatically.

## Job Queues

The API starts in-process workers for fast, medium, and slow jobs. These queues let Autofile process document-related work without blocking normal API requests.

## Processing Work

Background jobs handle work such as preview generation, text extraction, OCR, thumbnails, and document page processing.

## Operations

If document processing appears stuck, check:

- The API logs.
- Valkey connectivity.
- S3 bucket access.
