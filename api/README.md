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

### Default Admin User

On a fresh installation, Autofile creates a default admin user when no non-system users exist:

```text
username: admin
email: admin@example.com
password: admin123!
```

The default admin must change this password before using other authenticated features.

## Container Images

Image builds are managed from the repository root with Docker Buildx Bake. See the root README for current image and release commands.

## Testing

Run the test suite (install with `cargo install cargo-nextest` if needed):

```bash
cargo nextest run --locked
```

## Tech Stack

- **Web Framework**: Axum 0.8 with Tower middleware
- **Database**: PostgreSQL with Diesel-async and bb8 connection pooling
- **Authentication**: JWT tokens with Argon2 password hashing
- **File Storage**: AWS S3-compatible storage for document storage
- **Serialization**: Serde for JSON handling
