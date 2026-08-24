# Development

For most development, run the full Docker Compose stack from the repository root.

```bash
docker compose up --build
```

## API

From `api/`:

```bash
cargo run
```

The API expects PostgreSQL, Redis, S3 credentials, `S3_BUCKET`, and `JWT_SECRET` to be available in the environment.

Run API tests:

```bash
cargo test
```

## UI

From `ui/`:

```bash
npm install
npm run dev
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
mkdocs build --strict
```

Serve the documentation locally:

```bash
mkdocs serve
```
