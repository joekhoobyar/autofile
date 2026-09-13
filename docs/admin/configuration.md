# Configuration

The API is configured with environment variables.

| Variable | Required | Default | Description |
| --- | --- | --- | --- |
| `APP_MODE` | No | unset | Application mode. Docker Compose sets `development`. |
| `BIND_ADDR` | No | `0.0.0.0:8000` | API bind address. |
| `DATABASE_URL` | Yes | none | PostgreSQL connection string. |
| `REDIS_URL` | No | `redis://127.0.0.1:6379/?connect_timeout=2&timeout=2` | Redis connection string for background jobs. |
| `AWS_ENDPOINT_URL_S3` | No | AWS default | S3 endpoint override. Required for RustFS. |
| `AWS_ACCESS_KEY_ID` | Yes for RustFS/local | none | S3 access key. |
| `AWS_SECRET_ACCESS_KEY` | Yes for RustFS/local | none | S3 secret key. |
| `AWS_REGION` | Usually | AWS SDK default | S3 region. |
| `S3_BUCKET` | Yes | none | Bucket for document files. |
| `JWT_SECRET` | Yes | none | Secret used to sign JWTs. Use a strong value in production. |
| `ALLOWED_ORIGINS` | No | `http://localhost:5173` | Comma-separated CORS origins. |
| `MAX_UPLOAD_SIZE_MB` | No | `100` | Maximum size in megabytes for a single uploaded file. Applies to both `POST /api/v1/documents` and `POST /api/v1/documents/{document_id}/files`. Requires a restart to take effect. Invalid values prevent startup. Oversize uploads return `413 Payload Too Large`. |
| `RUST_LOG` | No | Rust tracing default | Logging filter, for example `info`. |

## Production Notes

- Use a strong, unique `JWT_SECRET`.
- Set `ALLOWED_ORIGINS` to the public UI origin.
- Use persistent PostgreSQL and object storage volumes or managed services.
- Back up PostgreSQL and the S3 bucket together so file metadata and file objects remain consistent.
