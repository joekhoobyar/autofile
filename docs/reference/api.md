# API Reference

Autofile exposes a REST API under `/api/v1`.

!!! note "Initial reference"
    This is a hand-written reference for the current API, not a generated OpenAPI specification. Resource coverage will expand as the API matures.

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

Create the first user through `/api/v1/auth/register`, then sign in through the UI or authentication API.

```bash
curl -i -X POST "http://localhost:8000/api/v1/auth/register" \
  -H "Content-Type: application/json" \
  -d '{
    "username": "admin@example.com",
    "email": "admin@example.com",
    "display_name": "Admin",
    "password": "changeme1234"
  }'
```

Passwords must contain at least 12 characters. Log in and save the refresh-token cookie with:

```bash
curl -i -c cookies.txt -X POST "http://localhost:8000/api/v1/auth/login" \
  -H "Content-Type: application/json" \
  -d '{
    "username": "admin@example.com",
    "password": "changeme1234"
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

Common statuses include `400 Bad Request`, `401 Unauthorized`, `404 Not Found`, `409 Conflict`, and `422 Unprocessable Entity`.

Malformed JSON and invalid path or query input can instead return an Axum framework rejection response.

## Health

- `GET /api/v1/health/ready`

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

## Metadata Resources

The metadata model spans four API resources:

- [Document Types](#document-types) classify documents.
- [Metadata Types](#metadata-types) define reusable fields.
- [Document Type Metadata Associations](#document-type-metadata-associations) select fields and required status for a Document Type.
- [Document Metadata](#document-metadata) stores values for a document.

See the [Metadata overview](../concepts/metadata.md) for the conceptual model and UI workflows.

## Document Types

Base path: `/api/v1/document-types`

### Resource

```json
{
  "id": 2,
  "slug": "invoice",
  "name": "Invoice",
  "description": "Supplier invoice",
  "created_by": 1,
  "created_at": "2026-08-25T12:00:00Z",
  "updated_by": 1,
  "updated_at": "2026-08-25T12:00:00Z"
}
```

### Endpoints

| Method | Path | Behavior |
| --- | --- | --- |
| `GET` | `/api/v1/document-types` | List Document Types. |
| `POST` | `/api/v1/document-types` | Create a Document Type. |
| `GET` | `/api/v1/document-types/{id}` | Get a Document Type by ID. |
| `PATCH` | `/api/v1/document-types/{id}` | Update its name or description. |
| `DELETE` | `/api/v1/document-types/{id}` | Delete it and reassign its documents to the default type. |
| `GET` | `/api/v1/document-types/by-slug/{slug}` | Get a Document Type by exact slug. |

List query parameters:

| Parameter | Description |
| --- | --- |
| `page` | 1-based page number. Defaults to `1`. |
| `per_page` | Items per page. Defaults to `50` and is limited to `1` through `200`. |
| `q` | Case-insensitive search of slug, name, and description. |
| `sf` | Sort field: `id`, `slug`, `name`, `description`, `created_at`, or `updated_at`. |
| `sd` | Set to `true` for descending order. |

Create a Document Type:

```http
POST /api/v1/document-types
Content-Type: application/json
Authorization: Bearer <access_token>

{
  "slug": "invoice",
  "name": "Invoice",
  "description": "Supplier invoice"
}
```

Update a Document Type:

```http
PATCH /api/v1/document-types/2
Content-Type: application/json
Authorization: Bearer <access_token>

{
  "name": "Supplier Invoice",
  "description": "Invoices received from suppliers"
}
```

The slug is required on creation, must be unique, and may contain only lowercase letters, numbers, hyphens, and underscores. It cannot be updated.

Deleting the default Document Type returns `400 Bad Request`. Deleting another type removes its metadata associations and reassigns its documents to the default type. It does not immediately remove all metadata values left on those documents.

See the [Document Types guide](../concepts/document-types.md).

## Metadata Types

Base path: `/api/v1/metadata-types`

### Resource

```json
{
  "id": 3,
  "slug": "status",
  "name": "Status",
  "data_type": "lookup",
  "description": "Invoice processing status",
  "options": {
    "choices": ["Open", "Paid", "Cancelled"]
  },
  "created_by": 1,
  "created_at": "2026-08-25T12:00:00Z",
  "updated_by": 1,
  "updated_at": "2026-08-25T12:00:00Z"
}
```

`data_type` must be `string`, `date`, or `lookup`. Lookup options use a `choices` array of strings. Options on other data types are ignored when document metadata is validated.

### Endpoints

| Method | Path | Behavior |
| --- | --- | --- |
| `GET` | `/api/v1/metadata-types` | List Metadata Types. |
| `POST` | `/api/v1/metadata-types` | Create a Metadata Type. |
| `GET` | `/api/v1/metadata-types/{id}` | Get a Metadata Type by ID. |
| `PATCH` | `/api/v1/metadata-types/{id}` | Update its mutable fields. |
| `DELETE` | `/api/v1/metadata-types/{id}` | Delete an unused Metadata Type. |
| `GET` | `/api/v1/metadata-types/by-slug/{slug}` | Get a Metadata Type by exact slug. |

List query parameters match Document Types. The supported `sf` values are `id`, `slug`, `name`, `data_type`, `description`, `created_at`, and `updated_at`. Search checks slug, name, data type, and description.

Create a String field:

```http
POST /api/v1/metadata-types
Content-Type: application/json
Authorization: Bearer <access_token>

{
  "slug": "correspondent",
  "name": "Correspondent",
  "data_type": "string",
  "description": "Person or organization that issued the document"
}
```

Create a Lookup field:

```http
POST /api/v1/metadata-types
Content-Type: application/json
Authorization: Bearer <access_token>

{
  "slug": "status",
  "name": "Status",
  "data_type": "lookup",
  "options": {
    "choices": ["Open", "Paid", "Cancelled"]
  }
}
```

`PATCH` accepts `name`, `data_type`, `description`, and `options`. The slug cannot be updated. Changing a data type or Lookup choices does not migrate or revalidate existing document values.

A Metadata Type cannot be deleted while document metadata rows reference it. The API removes Document Type associations as part of a successful deletion, but does not cascade deletion to document values.

See the [Metadata Types guide](../concepts/metadata-types.md).

## Document Type Metadata Associations

Base path: `/api/v1/document-types-metadata-types`

An association makes a Metadata Type available to a Document Type. Its `required` field applies only to that Document Type.

```json
{
  "document_type_id": 2,
  "metadata_type_id": 3,
  "required": true,
  "updated_at": "2026-08-25T12:00:00Z"
}
```

### Endpoints

| Method | Path | Behavior |
| --- | --- | --- |
| `GET` | `/api/v1/document-types-metadata-types` | List associations. |
| `POST` | `/api/v1/document-types-metadata-types` | Create one association. |
| `POST` | `/api/v1/document-types-metadata-types/{document_type_id}` | Replace all associations for a Document Type. |
| `GET` | `/api/v1/document-types-metadata-types/{document_type_id}/{metadata_type_id}` | Get one association. |
| `PATCH` | `/api/v1/document-types-metadata-types/{document_type_id}/{metadata_type_id}` | Change required status. |
| `DELETE` | `/api/v1/document-types-metadata-types/{document_type_id}/{metadata_type_id}` | Remove one association. |

The list accepts `page`, `per_page`, `q`, `document_type_id`, and `metadata_type_id`. Unlike other list endpoints, it returns a JSON array without a total count. Search checks the associated Metadata Type's slug, name, data type, and description.

Create one association:

```http
POST /api/v1/document-types-metadata-types
Content-Type: application/json
Authorization: Bearer <access_token>

{
  "document_type_id": 2,
  "metadata_type_id": 3,
  "required": true
}
```

Replace the complete association set for Document Type `2`:

```http
POST /api/v1/document-types-metadata-types/2
Content-Type: application/json
Authorization: Bearer <access_token>

[
  {"metadata_type_id": 1, "required": true},
  {"metadata_type_id": 2, "required": true},
  {"metadata_type_id": 3, "required": false}
]
```

!!! warning "Replacement operation"
    The Document Type-specific POST deletes all existing associations for that type and inserts the supplied array. An empty array removes every association. It does not delete existing document metadata values or verify that newly required values exist on current documents.

Removing an association can leave existing values stored on documents while making the field unavailable in the Metadata editor.

## Document Metadata

Base path: `/api/v1/documents/{document_id}/metadata`

One document can have at most one value for each Metadata Type.

```json
{
  "document_id": 10,
  "metadata_type_id": 2,
  "value": "2026-08-25",
  "created_at": "2026-08-25T12:00:00Z",
  "created_by": 1,
  "updated_at": "2026-08-25T12:00:00Z",
  "updated_by": 1
}
```

### Endpoints

| Method | Path | Behavior |
| --- | --- | --- |
| `GET` | `/api/v1/documents/{document_id}/metadata` | List stored metadata rows for a document. |
| `POST` | `/api/v1/documents/{document_id}/metadata` | Validate and upsert supplied values. |
| `GET` | `/api/v1/documents/{document_id}/metadata/{metadata_type_id}` | Get one stored value. |
| `DELETE` | `/api/v1/documents/{document_id}/metadata/{metadata_type_id}` | Delete one value if it is not required. |

Upsert metadata:

```http
POST /api/v1/documents/10/metadata
Content-Type: application/json
Authorization: Bearer <access_token>

[
  {"metadata_type_id": 1, "value": "Acme Corporation"},
  {"metadata_type_id": 2, "value": "2026-08-25"},
  {"metadata_type_id": 3, "value": "Paid"}
]
```

The response is an array containing all stored metadata rows for the document, ordered by Metadata Type ID.

!!! note "Incremental upsert"
    POST updates or inserts only the supplied fields. It does not replace the document's complete metadata set, and omitted fields remain unchanged. Sending an empty array makes no metadata changes.

Every submitted field must be associated with the document's current Document Type. Validation rules are:

| Data Type | Validation |
| --- | --- |
| `string` | Any string is accepted. |
| `date` | Non-empty values must be valid dates in `YYYY-MM-DD` format. |
| `lookup` | Non-empty values must match a configured choice after surrounding whitespace is trimmed for validation. |

A submitted required value cannot be empty. Optional empty strings are accepted and stored rather than deleting the row. Date and Lookup values are trimmed for validation, but the original submitted string is stored.

Required completeness is not checked across omitted fields. A document can therefore lack a required value if that field was never submitted. Deleting a currently required value returns `409 Conflict`.

Successful upserts and deletions queue updates for enabled document indexes that may depend on the changed metadata.

Document list and detail responses also expose stored values as a map keyed by Metadata Type slug:

```json
{
  "metadata": {
    "correspondent": "Acme Corporation",
    "issue_date": "2026-08-25",
    "status": "Paid"
  }
}
```

See the [Document Metadata guide](../concepts/document-metadata.md).
