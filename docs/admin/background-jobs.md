# Background Jobs

Autofile uses Redis-backed background jobs for document processing work.

## Redis

The API connects to Redis through `REDIS_URL`. The Docker Compose stack provides a Redis service automatically.

## Job Queues

The API starts in-process workers for fast, medium, and slow jobs. These queues let Autofile process document-related work without blocking normal API requests.

## Processing Work

Background jobs handle work such as preview generation, text extraction, OCR, thumbnails, and document page processing.

## Operations

If document processing appears stuck, check:

- The API logs.
- Redis connectivity.
- Availability of processing tools in the API runtime image.
- S3 bucket access.
