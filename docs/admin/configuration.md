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
| `MAX_UPLOAD_SIZE_MB` | No | `100` | Maximum size in megabytes for a single uploaded file. |
| `TMPDIR` | No | OS default (`/tmp`) | Scratch directory for uploads and document processing. Docker Compose and the Helm chart set `/tmp`. |
| `XDG_CACHE_HOME` | No | `~/.cache` | Cache directory for tool caches (fontconfig/Pango). Docker Compose and the Helm chart set `/tmp/.cache` so caches stay on ephemeral scratch. |
| `MAGICK_TMPDIR` | No | `TMPDIR` | ImageMagick pixel-cache directory. Docker Compose and the Helm chart set `/tmp`. |
| `RUST_LOG` | No | Rust tracing default: `error` | Logging filter. Possible values: `error`, `warn`, `info`, `debug`, `trace`, `off` |

## Production Notes

- Set `ALLOWED_ORIGINS` to the public UI origin, such as: `https://autofile.example.com`
- Use a strong, unique `JWT_SECRET`.
- Use persistent PostgreSQL.
- Use a self-hosted S3 object store (such as [RustFS](https://rustfs.com/) or [Garage](https://garagehq.deuxfleurs.fr/)), or use Amazon S3.
- Back up PostgreSQL and the S3 bucket together so file metadata and file objects remain consistent.
