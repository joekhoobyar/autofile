# API Reference

Autofile exposes a REST API under `/api/v1`.

!!! note "Initial reference"
    This is a route-family overview, not a generated OpenAPI reference. A generated API reference may be added after the Rust API adopts an OpenAPI generation workflow.

## Health

- `/api/v1/health/ready`

## Route Families

- `/api/v1/about`
- `/api/v1/auth`
- `/api/v1/cabinets`
- `/api/v1/classifier-blocks`
- `/api/v1/document-indexes`
- `/api/v1/document-types`
- `/api/v1/document-types-metadata-types`
- `/api/v1/documents`
- `/api/v1/metadata-types`
- `/api/v1/tags`
- `/api/v1/users`

## Authentication

Authentication uses JWT and cookies. Create the first user through `/api/v1/auth/register`, then sign in through the UI.

## Example Registration Request

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
