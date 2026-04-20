# Autofile API

REST API built with Axum and Diesel-async for document management with S3 storage.

## Running the API

```bash
cargo run
```

## Running with Docker Compose

From the repository root:

```bash
docker compose up --build
```

This starts:

- `api` (this service)
- `redis`
- `postgres`
- `minio` (plus a one-shot `minio-init` job to create bucket `autofile`)

### User Registration

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

## Pushing docker images to Harbor

Use these defaults:

- API image: harbor.k8s.khoobyar.name/joekhoobyar/autofile-api
- UI image: harbor.k8s.khoobyar.name/joekhoobyar/autofile-ui

### 1) Login (use your Harbor user or robot account)

```bash
docker login harbor.k8s.khoobyar.name
```

### 2) Common tags

```bash
GIT_SHA="$(git rev-parse --short HEAD)"
DATE_TAG="$(date +%Y%m%d)"
API_REPO="harbor.k8s.khoobyar.name/joekhoobyar/autofile-api"
UI_REPO="harbor.k8s.khoobyar.name/joekhoobyar/autofile-ui"
```

### 4) Build + push API (amd64 + arm64) with registry cache

```bash
docker buildx build \
  --platform linux/amd64,linux/arm64 \
  -f api/Dockerfile \
  -t "${API_REPO}:latest" \
  -t "${API_REPO}:${DATE_TAG}-${GIT_SHA}" \
  --cache-from type=registry,ref="${API_REPO}:buildcache" \
  --cache-to type=registry,ref="${API_REPO}:buildcache",mode=max \
  --push \
  .
```

### 5) Build + push UI (amd64 + arm64) with registry cache

```bash
docker buildx build \
  --platform linux/amd64,linux/arm64 \
  -f ui/Dockerfile \
  -t "${UI_REPO}:latest" \
  -t "${UI_REPO}:${DATE_TAG}-${GIT_SHA}" \
  --cache-from type=registry,ref="${UI_REPO}:buildcache" \
  --cache-to type=registry,ref="${UI_REPO}:buildcache",mode=max \
  --push \
  .
```

### 6) Verify multi-arch manifests were published

```bash
docker buildx imagetools inspect "${API_REPO}:${DATE_TAG}-${GIT_SHA}"
docker buildx imagetools inspect "${UI_REPO}:${DATE_TAG}-${GIT_SHA}"
```

## Testing

Run the test suite:

```bash
cargo test
```

## Tech Stack

- **Web Framework**: Axum 0.8 with Tower middleware
- **Database**: PostgreSQL with Diesel-async and bb8 connection pooling
- **Authentication**: JWT tokens with Argon2 password hashing
- **File Storage**: AWS S3 (or MinIO) for document storage
- **Serialization**: Serde for JSON handling
