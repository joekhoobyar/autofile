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
- `rustfs` (plus a one-shot `rustfs-init` job to create bucket `autofile-documents`)

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

## Container Images

Image builds are managed from the repository root with Docker Buildx Bake. See the root README for current image and release commands.

## Testing

Run the test suite:

```bash
cargo test
```

## Tech Stack

- **Web Framework**: Axum 0.8 with Tower middleware
- **Database**: PostgreSQL with Diesel-async and bb8 connection pooling
- **Authentication**: JWT tokens with Argon2 password hashing
- **File Storage**: AWS S3-compatible storage for document storage
- **Serialization**: Serde for JSON handling
