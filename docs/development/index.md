# Development

For most development, running the full Compose stack from the repository root is simpler:

```bash
docker compose up --build
```

See the [Docker Compose Quick Start](../getting-started/quick-start.md) for the services and local endpoints.

## API

Here is a sample set of environment variables:

```shell
# Autofile
export DATABASE_URL='postgres://autofile:@localhost:5432/autofile'
export JWT_SECRET='asuperduperupersecret'
export APP_MODE='development'
export RUST_LOG=info

# RustFS / Minio Configuration
export AWS_ENDPOINT_URL_S3='http://localhost:9000'
export S3_BUCKET='autofile-documents'
export AWS_ACCESS_KEY_ID='admin'
export AWS_SECRET_ACCESS_KEY='admin'
export AWS_REGION='us-east-1'
```

From `api/`:

```bash
cargo run
```

The API expects PostgreSQL, Redis, S3 credentials, `S3_BUCKET`, and `JWT_SECRET` to be available in the environment. For most development, running the full Compose stack is simpler.

Run API tests (install with `cargo install cargo-nextest --locked` if needed):

```bash
cargo nextest run --locked
```

## UI

From `ui/`:

```bash
npm install
VITE_API_HOST=http://localhost:8000 npm run dev
```

Test the UI:

```bash
npm test
```

Build the UI:

```bash
npm run build
```

Lint the UI:

```bash
npm run lint
```

## Documentation

Install documentation dependencies from the repository root:

```bash
pip install -r requirements-docs.txt
```

Build the documentation site:

```bash
zensical build --strict
```

Serve the documentation locally:

```bash
zensical serve
```

## Container Images

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

## Releases

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
