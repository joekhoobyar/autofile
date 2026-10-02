## 1. Database Schema

- [x] 1.1 Add a Diesel migration that introduces nullable `string_value`, `number_value NUMERIC`, and `date_value DATE` columns on `document_metadatas`.
- [x] 1.2 Backfill `string_value` for existing `string` and `lookup` metadata rows.
- [x] 1.3 Validate and backfill `date_value` for existing `date` metadata rows from `value::date`.
- [x] 1.4 Replace the `(metadata_type_id, value)` metadata suggestion index with `(metadata_type_id, string_value)`.
- [x] 1.5 Drop the old `value` column and add a check constraint requiring exactly one typed value column per row.
- [x] 1.6 Add a down migration that recreates `value`, backfills it from typed columns, removes typed columns, and restores the old index.
- [x] 1.7 Regenerate and review `api/src/schema.rs`.

## 2. Backend API And Domain

- [x] 2.1 Update `DocumentMetadata` to expose `string_value`, `number_value`, and `date_value` instead of `value`.
- [x] 2.2 Update `NewDocumentMetadata` to accept typed value fields instead of `value`.
- [x] 2.3 Implement typed metadata input normalization that rejects multiple non-null value fields with `422`.
- [x] 2.4 Validate that the submitted typed value field matches the Metadata Type data type, with lookup values stored in `string_value` and dates stored in `date_value`.
- [x] 2.5 Reject non-null `number_value` until a number Metadata Type exists.
- [x] 2.6 Preserve optional blank/null upsert behavior by deleting the stored row instead of writing an all-null row.
- [x] 2.7 Update bulk upsert conflict handling to set the selected typed value and clear the other typed value columns.

## 3. Backend Reads And Search

- [x] 3.1 Update document metadata list/get endpoints to select and return typed metadata rows.
- [x] 3.2 Add a helper for converting stored typed metadata to the string form used by `DocumentView.metadata`.
- [x] 3.3 Update document list and detail metadata loading to populate `DocumentView.metadata` from typed columns.
- [x] 3.4 Update classifier persistence to route slug-keyed string metadata actions through typed metadata normalization so Date metadata actions parse as `YYYY-MM-DD` and store in `date_value`.
- [x] 3.5 Update document index template inputs to continue receiving slug-keyed string metadata.
- [x] 3.6 Change `metadata_value` document search filtering to query only `string_value`.
- [x] 3.7 Keep `metadata_type_id`-only document search matching any stored metadata row for that type.
- [x] 3.8 Update metadata type value suggestions to query distinct non-empty `string_value` values.

## 4. UI

- [ ] 4.1 Update handwritten document metadata models to use `string_value`, `number_value`, and `date_value`.
- [ ] 4.2 Update metadata editor row utilities to build typed upsert payloads from row data types.
- [ ] 4.3 Send `string_value` for string and lookup rows, `date_value` for date rows, and no `number_value` until number metadata is supported.
- [ ] 4.4 Update UI tests that assert document metadata save payloads.
- [ ] 4.5 Update advanced search UI copy if needed to clarify that metadata value search matches string values.

## 5. Documentation And OpenAPI

- [ ] 5.1 Update Rust OpenAPI schema comments and endpoint descriptions for typed metadata row payloads.
- [ ] 5.2 Update `docs/concepts/document-metadata.md` for typed storage, typed API payloads, and string-only metadata value search.
- [ ] 5.3 Update `docs/reference/api.md` for the breaking metadata API change and unchanged `DocumentView.metadata` string map.
- [ ] 5.4 Update any classifier or index documentation that references metadata value serialization if needed.

## 6. Tests And Validation

- [ ] 6.1 Update integration test fixtures to insert typed metadata values.
- [ ] 6.2 Add or update API tests for saving and reading string metadata rows.
- [ ] 6.3 Add or update API tests for saving and reading date metadata rows as typed rows and document view strings.
- [ ] 6.4 Add API tests that multiple typed value fields return `422`.
- [ ] 6.5 Add API tests that non-null `number_value` returns `422` until number metadata exists.
- [ ] 6.6 Add API tests that `metadata_value` search matches `string_value` and does not match `date_value`.
- [ ] 6.7 Add API tests that `metadata_type_id`-only search still matches date metadata rows.
- [ ] 6.8 Add API tests that classifier metadata actions can write valid `YYYY-MM-DD` Date metadata into `date_value` and reject invalid date strings.
- [ ] 6.9 Run relevant API checks from `api/`: `cargo fmt --all -- --check`, `cargo check --locked --all-targets`, and targeted tests.
- [ ] 6.10 Run relevant UI checks from `ui/`: `npm run lint`, `npm test`, and `npm run build` if UI files change.
