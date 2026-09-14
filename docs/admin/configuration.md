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
| `MALWARE_SCANNER_PROVIDER` | No | `disabled` | Malware scanner provider: `clamav`, `external` (external `clamd` endpoint), or `disabled`. Invalid values prevent startup. |
| `CLAMAV_HOST` | No | `clamav` | Hostname of the ClamAV `clamd` endpoint. Used when the provider is `clamav` or `external`. |
| `CLAMAV_PORT` | No | `3310` | Port of the ClamAV `clamd` endpoint. Invalid values prevent startup. |
| `CLAMAV_CONNECT_TIMEOUT_SECONDS` | No | `5` | Timeout in seconds for connecting to `clamd`. Invalid values prevent startup. |
| `CLAMAV_SCAN_TIMEOUT_SECONDS` | No | `120` | Timeout in seconds for a single scan. Invalid values prevent startup. |
| `MALWARE_SCANNER_FAILURE_POLICY` | No | `closed` | Scanner failure policy: `closed` marks failed scans as `scan_error` and blocks file use, `open` marks them `not_required` so processing continues. |
| `TMPDIR` | No | OS default (`/tmp`) | Scratch directory for uploads and document processing. Docker Compose and the Helm chart set `/tmp`. |
| `XDG_CACHE_HOME` | No | `~/.cache` | Cache directory for tool caches (fontconfig/Pango). Docker Compose and the Helm chart set `/tmp/.cache` so caches stay on ephemeral scratch. |
| `MAGICK_TMPDIR` | No | `TMPDIR` | ImageMagick pixel-cache directory. Docker Compose and the Helm chart set `/tmp`. |
| `RUST_LOG` | No | Rust tracing default: `error` | Logging filter. Possible values: `error`, `warn`, `info`, `debug`, `trace`, `off` |

## Virus Scanning

Uploaded document files can be scanned for malware before they are previewed, downloaded, or processed. Scanning has two layers:

- **Deployment configuration** (environment variables above) points the API at a ClamAV `clamd` endpoint and selects the failure policy.
- **Application settings** control product behavior: `virus_scanning_enabled` turns scanning on, and `virus_scan_by_default` sets the default state of the per-upload `Virus scan this upload` checkbox. Administrators manage these in the UI settings page; upload pages read them from `GET /api/v1/public/settings`.

When a scan is requested, the file is stored in object storage immediately but marked unavailable (`scan_status` `pending`, then `scanning`) until the scan job verdict arrives:

| Scan status | Meaning |
| --- | --- |
| `not_required` | Scanning was disabled or the uploader opted out. The file is usable. |
| `pending` / `scanning` | Waiting for or undergoing a scan. The file is unavailable. |
| `clean` | Scanner accepted the file. The file is usable. |
| `infected` | Scanner found a threat (`scan_threat_name` records it when available). The file stays blocked but is retained for audit; deleting the file removes the object. |
| `scan_error` | The scan failed or timed out. The file stays blocked until rescanned. |

Document-file responses include these fields plus a derived `content_available` boolean (true only for `clean` and `not_required`). Downloads, previews, page images, extracted text, OCR text, thumbnails, page processing, and classification all require `content_available=true`. Files can still be deleted, and eligible files can be submitted for rescan with `POST /api/v1/documents/{document_id}/files/{id}/rescan`.

Scanner failures default to fail-closed: the file is marked `scan_error` and never processed. With `MALWARE_SCANNER_FAILURE_POLICY=open`, failures are marked `not_required` so processing continues; prefer `closed` unless scanner downtime must not block ingestion.

For local testing, Docker Compose provides a `clamav` service and sets the scanner environment variables on the API. ClamAV may take several minutes to download and load signatures on first startup, so early scans can fail until it is ready. Keep `clamd` on the private deployment network: the protocol is unauthenticated and unencrypted, and Autofile streams file bytes to it (the scanner never receives object-storage credentials).

## Production Notes

- Set `ALLOWED_ORIGINS` to the public UI origin, such as: `https://autofile.example.com`
- Use a strong, unique `JWT_SECRET`.
- Use persistent PostgreSQL.
- Use a self-hosted S3 object store (such as [RustFS](https://rustfs.com/) or [Garage](https://garagehq.deuxfleurs.fr/)), or use Amazon S3.
- Back up PostgreSQL and the S3 bucket together so file metadata and file objects remain consistent.
