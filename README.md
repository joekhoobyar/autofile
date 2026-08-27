# Autofile

Autofile is a self-hosted document management application. It stores document files in S3-compatible object storage, keeps structured document metadata in PostgreSQL, and provides a React web UI backed by a Rust API.

For installation guides, concepts, configuration, and API reference, see the full documentation at [autofile.dev](https://autofile.dev/).

The project is built as two containers:

- `autofile-api`: Rust/Axum API, background workers, PostgreSQL migrations, S3 file storage, and document processing tools.
- `autofile-ui`: React/Vite frontend served by nginx.

## ✨ Features

- Upload and manage documents and document files.
- Organize documents with cabinets and tags.
- Define document types and metadata types.
- Build document indexes and index templates.
- Run classifier blocks for document classification workflows.
- Store files in S3-compatible storage such as RustFS.
- Process document previews, text, OCR, and thumbnails through background jobs.
- Manage users with JWT/cookie-based authentication.

## 🧰 Tech Stack

- API: Rust, Axum, Tokio, Diesel, Diesel Async, Apalis
- UI: React, TypeScript, Vite, PrimeReact, TanStack Query
- Data: PostgreSQL, Redis, S3-compatible object storage
- Local services: Docker Compose, RustFS
- Images: Docker Buildx/Bake, multi-platform `linux/amd64` and `linux/arm64`

## 🚀 Installation

Autofile supports two installation methods:

- Helm for Kubernetes deployments.
- Docker Compose for local development and evaluation.

Install the published Helm chart from GHCR:

```bash
CHART_VERSION="<chart-version>"
helm upgrade --install autofile oci://ghcr.io/joekhoobyar/charts/autofile \
  --version "${CHART_VERSION}"
```

Run locally with Docker Compose from the repository root:

```bash
docker compose up
```

For complete installation instructions, see [Installation](https://autofile.dev/getting-started/installation/).

## 🐳 Docker Compose Quick Start

This starts:

- PostgreSQL on the internal Compose network
- Redis on the internal Compose network
- RustFS at `http://localhost:9000`
- RustFS console at `http://localhost:9001`
- API at `http://localhost:8000`
- UI at `http://localhost:5173`

RustFS uses the default local credentials from `docker-compose.yml`:

```text
username: autofile-dev-access
password: autofile-dev-secret
```

The `rustfs-init` service creates the local bucket `autofile-documents` automatically.

## 👤 Create the First User

After the API is running, create a user through the registration endpoint:

```bash
curl -i -X POST "http://localhost:8000/api/v1/auth/register" \
  -H "Content-Type: application/json" \
  -d '{
    "username": "admin@example.com",
    "email": "admin@example.com",
    "display_name": "Admin",
    "password": "changeme123"
  }'
```

Then open the UI at `http://localhost:5173` and sign in.

## ⚙️ Configuration

The API is configured with environment variables.

| Variable | Required | Default | Description |
| --- | --- | --- | --- |
| `APP_MODE` | No | unset | Application mode. Compose sets `development`. |
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
| `RUST_LOG` | No | Rust tracing default | Logging filter, for example `info`. |

## 🛠️ Development

### API

From `api/`:

```bash
cargo run
```

The API expects PostgreSQL, Redis, S3 credentials, `S3_BUCKET`, and `JWT_SECRET` to be available in the environment. For most development, running the full Compose stack is simpler.

Run API tests:

```bash
cd api
cargo test
```

### UI

From `ui/`:

```bash
npm install
npm run dev
```

Build the UI:

```bash
cd ui
npm run build
```

Lint the UI:

```bash
cd ui
npm run lint
```

## 📦 Container Images

The project uses Docker Buildx Bake through `docker-bake.hcl`.

The API image is built on two reusable base images:

- `autofile-api-rust-base`: Rust toolchain, native build dependencies, and `cargo-chef`.
- `autofile-api-runtime-base`: runtime document-processing dependencies such as LibreOffice, Pandoc, Poppler, Tesseract, WeasyPrint, TeX, and `tini`.

Build and push both API base images:

```bash
make image TARGET=base
```

Build and push just one base image:

```bash
make image TARGET=autofile-api-rust-base
make image TARGET=autofile-api-runtime-base
```

Builds are single-platform. By default, Bake uses `ARCH=amd64`, which maps to `linux/amd64`; CI also builds with `ARCH=arm64` on a native ARM64 runner.

Build and push the default ad-hoc image set:

```bash
make image
```

The Makefile tags ad-hoc images with the short Git SHA and architecture suffix, for example `<git-sha>-amd64`. If the working tree is dirty, it appends `-dirty` before the architecture suffix.

Build a release image set manually:

```bash
RELEASE_TAG=v0.2.0 make image TARGET=release
```

The release group publishes architecture-specific tags for each image:

- `latest-amd64` or `latest-arm64`
- the release tag plus architecture suffix, for example `v0.2.0-amd64`
- the Git SHA plus architecture suffix

By default, the bake file publishes images to GHCR under `ghcr.io/joekhoobyar`. Override the repo variables if you want to publish to another registry.

Final API builds use `BASE_TAG=latest` by default for both API base images, resolved as architecture-specific base tags such as `latest-amd64` or `latest-arm64`.

## 🏷️ Releases

GitHub Actions builds and pushes images from `.github/workflows/release.yml`.

The workflow builds `amd64` and `arm64` images on native GitHub-hosted runners, then publishes multi-architecture manifest tags in a final job.

Release builds run when a Git tag matching `v*` is pushed:

```bash
git tag v0.2.0
git push origin v0.2.0
```

Tag-triggered release builds push:

- `ghcr.io/joekhoobyar/autofile-api:latest`
- `ghcr.io/joekhoobyar/autofile-api:<tag>`
- `ghcr.io/joekhoobyar/autofile-api:<git-sha>`
- `ghcr.io/joekhoobyar/autofile-ui:latest`
- `ghcr.io/joekhoobyar/autofile-ui:<tag>`
- `ghcr.io/joekhoobyar/autofile-ui:<git-sha>`

Manual workflow dispatch can build ad-hoc app images, both API base images, or either API base image individually. Ad-hoc app builds push architecture-specific Git SHA tags and a final multi-architecture Git SHA manifest tag.

## 📄 License

Autofile is licensed under the GNU Affero General Public License v3.0 only. See [LICENSE](LICENSE).
