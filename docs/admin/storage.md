# Storage

Autofile stores document file bytes in S3-compatible object storage and stores document metadata in PostgreSQL.

## S3-Compatible Storage

Autofile uses an S3-compatible API to store documents. For local development, Docker Compose runs RustFS and configures the API with `AWS_ENDPOINT_URL_S3`.

## Local Bucket

The Docker Compose stack creates this bucket automatically:

```text
autofile-documents
```

## RustFS Settings

For local Compose, the API uses path-style requests for the local RustFS endpoint.

## Backups

Back up PostgreSQL and the S3 bucket as a matching set. PostgreSQL contains the document and file records; object storage contains the uploaded file contents.
