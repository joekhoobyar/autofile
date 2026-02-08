# Autofile API

REST API built with Axum and Diesel-async for document management with S3 storage.

## Running the API

```bash
cargo run
```

## Testing

Run the integration test suite:

```bash
bash test_simple.sh
```

## Tech Stack

- **Web Framework**: Axum 0.8 with Tower middleware
- **Database**: PostgreSQL with Diesel-async and bb8 connection pooling
- **Authentication**: JWT tokens with Argon2 password hashing
- **File Storage**: AWS S3 (or MinIO) for document storage
- **Serialization**: Serde for JSON handling
