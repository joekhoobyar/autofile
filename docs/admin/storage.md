# Storage

Autofile stores document file bytes in S3-compatible object storage and stores document metadata in PostgreSQL.

## S3-Compatible Storage

Autofile uses the AWS S3 API. For local development, Docker Compose runs MinIO and configures the API with `AWS_ENDPOINT_URL_S3`.

## Local Bucket

The Docker Compose stack creates this bucket automatically:

```text
autofile-documents
```

## MinIO Settings

For local Compose, the API uses path-style requests because they are required by MinIO in this setup.

## Backups

Back up PostgreSQL and the S3 bucket as a matching set. PostgreSQL contains the document and file records; object storage contains the uploaded file contents.
