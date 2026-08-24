# Quick Start

The easiest way to run Autofile locally is Docker Compose from the repository root.

```bash
docker compose up --build
```

This starts:

- PostgreSQL on the internal Compose network.
- Redis on the internal Compose network.
- MinIO at `http://localhost:9000`.
- MinIO console at `http://localhost:9001`.
- API at `http://localhost:8000`.
- UI at `http://localhost:5173`.

## MinIO Credentials

The local MinIO service uses the default credentials from `docker-compose.yml`:

```text
username: minioadmin
password: minioadmin
```

The `minio-init` service creates the local bucket `autofile-documents` automatically.

## Next Step

After the stack is running, [create the first user](first-user.md), then open `http://localhost:5173` and sign in.
