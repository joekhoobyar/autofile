# API Reference

Autofile exposes a REST API under `/api/v1`.

The detailed endpoint contract is generated from the Rust API source and is available from your running server:

- `GET /api/v1/openapi.json` for the raw OpenAPI document.
- `/api/docs` for interactive Swagger UI.
- [OpenAPI Spec](openapi.md) for the latest released specification embedded in this documentation site.

Use this page for high-level integration guidance, authentication behavior, common response shapes, and links into the relevant concept docs. Use the OpenAPI specification for request bodies, response schemas, query parameters, and status codes.

## Authentication

Most API routes require an authenticated user. Login returns a short-lived JWT access token in the response body and sets a longer-lived refresh token in an HTTP-only cookie. Send the access token with protected requests as `Authorization: Bearer <access_token>`.

The public endpoints are:

- `GET /api/v1/health/ready`
- `GET /api/v1/about`
- `GET /api/v1/about/license`
- `POST /api/v1/auth/register`
- `POST /api/v1/auth/login`
- `POST /api/v1/auth/refresh`, using the refresh-token cookie
- `POST /api/v1/auth/logout`
- `GET /api/v1/public/settings`

On a fresh installation, Autofile creates a default admin user when no non-system users exist:

```text
username: admin
email: admin@example.com
password: admin123!
```

Change the default password before using Autofile in any shared or persistent environment. The default admin is required to change this password before using other authenticated APIs.

Log in and save the refresh-token cookie with:

```bash
curl -i -c cookies.txt -X POST "http://localhost:8000/api/v1/auth/login" \
  -H "Content-Type: application/json" \
  -d '{
    "username": "admin",
    "password": "admin123!"
  }'
```

The response supplies the token for protected requests:

```json
{
  "access_token": "<access_token>",
  "token_type": "Bearer",
  "expires_in": 3600
}
```

New users can also be created through `POST /api/v1/auth/register` when user registration is enabled. Registered users receive the `user` role and are disabled until an admin enables them. Registration passwords must contain at least 12 characters.

## Common Responses

Paginated resource lists use this shape:

```json
{
  "total": 42,
  "page": 1,
  "per_page": 50,
  "items": []
}
```

Application errors contain a message and use the relevant HTTP status:

```json
{
  "message": "Metadata field 3 must be a valid date in YYYY-MM-DD format"
}
```

Common statuses include `400 Bad Request`, `401 Unauthorized`, `403 Forbidden`, `404 Not Found`, `409 Conflict`, and `422 Unprocessable Entity`.

When a user must change their password, authenticated endpoints other than `POST /api/v1/profile/password` return `403 Forbidden`:

```json
{
  "message": "Password change required",
  "code": "password_change_required"
}
```

Malformed JSON and invalid path or query input can instead return an Axum framework rejection response.

## Health

- `GET /api/v1/health/ready` is public and reports whether the API is ready to serve traffic.
- `GET /api/v1/ping` requires authentication and returns `{}`. Use it to verify that the current access token is still accepted.

## Route Families

Autofile groups endpoints by resource family:

- `/api/v1/about`
- `/api/v1/auth`
- `/api/v1/cabinets`
- `/api/v1/classifier-blocks`
- `/api/v1/document-indexes`
- `/api/v1/document-types`
- `/api/v1/document-types-metadata-types`
- `/api/v1/documents`
- `/api/v1/metadata-types`
- `/api/v1/ping`
- `/api/v1/profile`
- `/api/v1/public`
- `/api/v1/settings`
- `/api/v1/tags`
- `/api/v1/users`

The [OpenAPI Spec](openapi.md) lists the methods, parameters, request bodies, and response schemas for each family.

## Documents

Documents are the main managed resource and may include a stored file, extracted text, OCR text, classifier results, metadata, tags, cabinet placement, and document index assignments.

See the [OpenAPI Spec](openapi.md) for document endpoint details and the [Documents concept guide](../concepts/documents.md) for how documents are processed.

## Settings

Settings endpoints under `/api/v1/settings` require an admin access token. Public settings are exposed separately under `/api/v1/public/settings` through an explicit allowlist that is safe for anonymous clients.

`allow_user_registration` controls whether unauthenticated users can call `POST /api/v1/auth/register`. Date and datetime format settings control frontend display only; API date metadata values remain `YYYY-MM-DD`, and API timestamps remain serialized as timestamp strings.

See the [OpenAPI Spec](openapi.md) for settings payloads and responses.

## Profile

Profile endpoints operate on the current authenticated user. When `force_password_change` is set, only `POST /api/v1/profile/password` is available until the password is changed.

A successful password change clears `force_password_change`, returns a fresh access token, and refreshes the session cookie. New passwords must contain at least 12 characters.

See the [OpenAPI Spec](openapi.md) for profile payloads and responses.

## Users

User management endpoints require an admin access token. Admins can enable or disable users, require password changes, update roles, soft-delete users, and restore deleted users as disabled.

The system user cannot be updated, deleted, or restored. An admin cannot change their own role from `admin` to `user`, disable their own account, or delete their own account.

See the [OpenAPI Spec](openapi.md) for user endpoint details.

## Classifier Rule Validation

`POST /api/v1/classifier-blocks/validate` accepts a classifier `rules` object and returns semantic validation without saving it. Classifier create and update requests enforce the same validation and return `422 Unprocessable Entity` for invalid rules.

See the [Classifier Blocks guide](../concepts/classifier-blocks.md), [Classifier Rules YAML reference](classifier-rules-yaml.md), and [OpenAPI Spec](openapi.md) for details.

## Metadata Resources

The metadata model spans four API resource families:

- [Document Types](#document-types) classify documents.
- [Metadata Types](#metadata-types) define reusable fields.
- [Document Type Metadata Associations](#document-type-metadata-associations) select fields and required status for a Document Type.
- [Document Metadata](#document-metadata) stores values for a document.

See the [Metadata overview](../concepts/metadata.md) for the conceptual model and UI workflows. See the [OpenAPI Spec](openapi.md) for endpoint contracts.

## Document Types

Document Types classify documents and determine which metadata fields are available for a document. Creating, updating, and deleting Document Types requires an admin user.

The slug is required on creation, must be unique, and may contain only lowercase letters, numbers, hyphens, and underscores. It cannot be updated.

See the [Document Types guide](../concepts/document-types.md) and [OpenAPI Spec](openapi.md).

## Metadata Types

Metadata Types define reusable fields. Their `data_type` is `string`, `date`, or `lookup`; lookup options use a `choices` array of strings.

Creating, updating, and deleting Metadata Types requires an admin user. Changing a data type or lookup choices does not migrate or revalidate existing document values.

See the [Metadata Types guide](../concepts/metadata-types.md) and [OpenAPI Spec](openapi.md).

## Document Type Metadata Associations

An association makes a Metadata Type available to a Document Type. Its `required` field applies only to that Document Type.

Creating, replacing, updating, and deleting associations requires an admin user. Replacing the complete association set for a Document Type does not delete existing document metadata values or verify that newly required values exist on current documents.

See the [Document Types guide](../concepts/document-types.md), [Metadata overview](../concepts/metadata.md), and [OpenAPI Spec](openapi.md).

## Document Metadata

Document Metadata stores per-document values keyed by Metadata Type. One document can have at most one value for each Metadata Type.

Date metadata values must be sent as `YYYY-MM-DD`, regardless of the frontend display format. Lookup values must match a configured choice after surrounding whitespace is trimmed for validation.

Metadata upserts are incremental: omitted fields remain unchanged, and blank optional values delete the stored row instead of storing an empty string. Successful metadata changes queue updates for enabled document indexes that may depend on the changed metadata.

See the [Document Metadata guide](../concepts/document-metadata.md) and [OpenAPI Spec](openapi.md).
