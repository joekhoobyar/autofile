## Why

Document metadata currently stores every value in a single string column, even when the metadata type is a date. This makes storage less accurate, limits future typed behavior, and causes metadata value search to treat dates like free-form text.

## What Changes

- **BREAKING**: Change the document metadata API request and response shape from a single `value` field to explicit typed value fields: `string_value`, `number_value`, and `date_value`.
- Store `string` and `lookup` metadata in `document_metadatas.string_value`.
- Store `date` metadata in `document_metadatas.date_value` as a PostgreSQL `DATE`.
- Add nullable `document_metadatas.number_value` backed by PostgreSQL `NUMERIC`; number metadata is not introduced or accepted yet.
- Reject document metadata upsert payloads that set multiple typed value fields with `422 Unprocessable Entity`.
- Keep `DocumentView.metadata` as a slug-keyed `HashMap<String, String>` display/template representation, with date values serialized as `YYYY-MM-DD`.
- Change advanced metadata value search to search only `string_value` for this change.
- Keep metadata type value suggestions limited to string metadata types and backed by `string_value`.
- Update UI models and metadata edit payload construction to use typed value fields.
- Update documentation and tests for the new API contract, typed storage behavior, and string-only metadata value search.

## Capabilities

### New Capabilities
- `document-metadata`: Defines document metadata typed value storage, API behavior, validation, search semantics, and document view representation.

### Modified Capabilities

## Impact

- Database: new Diesel migration for typed columns, backfill, check constraints, and metadata value suggestion indexes; regenerate `api/src/schema.rs`.
- API: breaking changes to `DocumentMetadata` and document metadata upsert payloads under `/api/v1/documents/{document_id}/metadata`.
- Backend: update validation, upsert, read mapping, document list/detail metadata loading, classifier persistence, index template inputs, advanced search, and metadata value suggestions.
- UI: update handwritten models, metadata editor save payloads, document metadata tests, and any typed API assumptions.
- Documentation: update document metadata concepts, API reference, and OpenAPI comments.
- Tests: update integration fixtures and add coverage for typed API validation, date storage/readback, string-only metadata value search, and metadata value suggestions.
