## Context

Autofile document metadata is currently stored in `document_metadatas.value` as a required string regardless of the metadata type. Date metadata is validated as `YYYY-MM-DD`, but it is still persisted as text. Lookup metadata is also text. Metadata type value suggestions and advanced metadata value search both query the string column directly.

The UI already treats Date metadata specially at the editor boundary: it displays dates using the configured frontend date format and serializes edited dates back to `YYYY-MM-DD`. The API and backend currently expose document metadata rows as `{ metadata_type_id, value }`, while `DocumentView.metadata` exposes a slug-keyed `HashMap<String, String>` used by document detail responses, templates, classifiers, and document index logic.

This change splits stored metadata values by type and intentionally makes the document metadata row API typed. `DocumentView.metadata` remains a string map because it is a display/template representation and there is no obvious ergonomic typed alternative for templates or classifier matching.

## Goals / Non-Goals

**Goals:**

- Replace `document_metadatas.value` with typed storage columns: `string_value`, `number_value`, and `date_value`.
- Store Date metadata as PostgreSQL `DATE` and serialize it as `YYYY-MM-DD` where a string representation is still required.
- Store String and Lookup metadata in `string_value`.
- Add nullable `number_value NUMERIC` storage and expose `number_value: number | null` in the API, while rejecting non-null number metadata until a number metadata type exists.
- Make document metadata row APIs explicit and typed by replacing `value` with typed value fields.
- Reject ambiguous upsert payloads that set multiple typed value fields with `422 Unprocessable Entity`.
- Restrict advanced metadata value search to `string_value` for this change.
- Preserve the existing `DocumentView.metadata: HashMap<String, String>` contract.

**Non-Goals:**

- Introduce a new `number` metadata type.
- Add numeric range search, date range search, or typed advanced search operators.
- Change classifier rule syntax or index template syntax.
- Change frontend date display settings or backend timestamp serialization.
- Preserve backward compatibility for clients sending or reading `value` on document metadata row endpoints.

## Decisions

### Use Typed Columns In `document_metadatas`

`document_metadatas` will store exactly one non-null typed value per row:

- `string_value VARCHAR NULL`
- `number_value NUMERIC NULL`
- `date_value DATE NULL`

A database check constraint will enforce `num_nonnulls(string_value, number_value, date_value) = 1`. This keeps the invariant durable even if data is written outside the application.

Alternatives considered:

- Keep a single string column and add typed generated columns. This would preserve compatibility but keep the string column as the source of truth.
- Use JSONB for values. This would avoid multiple nullable columns but weaken type constraints and indexing clarity.
- Use one table per metadata value type. This would make type constraints explicit but complicate joins, upserts, and document metadata listing.

### Make Document Metadata Row APIs Typed And Breaking

Document metadata row responses will expose typed columns instead of `value`:

```json
{
  "document_id": 10,
  "metadata_type_id": 3,
  "string_value": null,
  "number_value": null,
  "date_value": "2026-08-25",
  "created_at": "...",
  "created_by": 1,
  "updated_at": "...",
  "updated_by": 1
}
```

Upsert requests will use the same typed value fields:

```json
{
  "metadata_type_id": 3,
  "date_value": "2026-08-25"
}
```

Payloads that set more than one typed value field will return `422`. Payloads whose non-null field does not match the metadata type will also return `422`. Blank or null-only optional values will continue to delete the stored metadata row.

Alternatives considered:

- Preserve `{ value: string }` externally and map typed storage internally. This would minimize client changes but hide the new typed model and keep the API semantically misleading.
- Accept both `value` and typed fields during a transition. This would reduce migration friction but add compatibility logic without a concrete requirement.
- Silently choose the value field matching the metadata type when multiple fields are set. This would hide client bugs, so the API will reject ambiguous input.

### Keep `DocumentView.metadata` As A String Map

`DocumentView.metadata` will remain `HashMap<String, String>` keyed by metadata type slug. It is used as a display/template/classifier-friendly representation rather than the canonical metadata row shape.

Typed database values will be converted to strings when building `DocumentView.metadata`:

- `string_value` returns as-is.
- `date_value` serializes as `YYYY-MM-DD`.
- `number_value` is not expected yet; future number metadata can define its serialization rules before enabling writes.

Alternatives considered:

- Change `DocumentView.metadata` to a typed object map. This would be more structurally accurate but would complicate templates, classifier matching, existing UI rendering, and any slug-keyed integrations.
- Return both typed metadata rows and the string map in `DocumentView`. This would duplicate data and increase payload complexity without a current use case.

### Store Lookup Values In `string_value`

Lookup metadata values will use `string_value`. Lookup values are selected strings constrained by metadata type options, so they fit the string storage path and can share string search and suggestion behavior where applicable.

### Route Classifier Metadata Through Typed Validation

Classifier block actions will continue to produce metadata values as strings keyed by metadata type slug. When persisted, classifier metadata must flow through the same typed metadata validation and normalization path as user/API metadata upserts.

For Date metadata, classifier-produced strings must be parsed as `YYYY-MM-DD` and stored in `date_value`. Invalid classifier-produced date strings must fail classification persistence with the same validation behavior as an invalid user-submitted date. This preserves the existing classifier rule syntax while ensuring classifier writes respect typed storage.

Alternatives considered:

- Require classifier rules to emit typed value objects. This would make classifier rules more explicit but would be a larger syntax break and unnecessary because metadata type definitions already provide the target type.
- Store classifier outputs as strings regardless of target type. This would bypass the new typed storage invariant and break date metadata consistency.

### Keep `number_value` Nullable And JSON Numeric

`number_value` will be nullable in the database and API. The TypeScript model will use `number | null` because product semantics are simple numeric metadata rather than arbitrary precision accounting or identifier values.

The backend will not accept non-null `number_value` until a number metadata type is introduced. This avoids storing values that no metadata type can validate or display correctly.

### Limit Advanced Metadata Value Search To Strings

The `metadata_value` document list filter will search only `document_metadatas.string_value` using the current case-insensitive partial match behavior. It will no longer match date metadata values.

Selecting `metadata_type_id` without `metadata_value` will continue to find documents that have a stored row for that metadata type, regardless of which typed value column is populated.

Typed date or numeric search can be added later with explicit query parameters and operators.

### Use A Guarded Date Backfill

The migration will backfill date metadata with `date_value = value::date`. PostgreSQL accepts `YYYY-MM-DD` as an ISO date literal. The migration should still guard or fail clearly on any historical date metadata value that does not match the expected format, because a single invalid cast would abort the migration.

String and lookup metadata will backfill to `string_value = value`.

## Risks / Trade-offs

- Breaking API clients → Document the new typed request/response shape and update the UI models in the same change.
- Invalid historical date values could abort migration → Add preflight validation or guarded migration logic that fails with a clear error before casting.
- Check constraint can conflict with delete semantics → Preserve the existing behavior that blank/null-only optional upsert payloads delete rows instead of writing an all-null row.
- `number_value` is exposed before number metadata exists → Reject non-null `number_value` until the number metadata type is introduced.
- `DocumentView.metadata` remains less structurally accurate than row APIs → Treat it as a display/template representation and document its string serialization rules.
- String-only `metadata_value` search may surprise users who previously searched dates as text → Update advanced search documentation and UI copy if needed.

## Migration Plan

1. Add a Diesel migration that creates `string_value`, `number_value`, and `date_value` as nullable columns.
2. Backfill `string_value` for metadata types with `data_type IN ('string', 'lookup')`.
3. Validate date metadata values match `YYYY-MM-DD` and are castable as dates.
4. Backfill `date_value` for metadata types with `data_type = 'date'` using `value::date`.
5. Replace the existing metadata value suggestion index on `(metadata_type_id, value)` with an index on `(metadata_type_id, string_value)`.
6. Drop the old `value` column.
7. Add the `num_nonnulls(...) = 1` check constraint.
8. Regenerate `api/src/schema.rs`.
9. Update backend models, upsert validation, reads, advanced search, suggestions, classifiers, and document index inputs.
10. Update UI models and metadata edit payload construction.
11. Update documentation and tests.

Rollback migration will recreate `value VARCHAR`, backfill from the populated typed column, drop the typed columns, and recreate the old index. Rolling back after future number metadata exists may require explicit number string serialization rules, but this change does not introduce persisted number metadata.

## Open Questions

- Should the UI advanced search label/help text explicitly say “string metadata value” or is documentation sufficient?
- Should metadata row endpoints return typed fields with omitted unset fields or explicit `null` fields? This design assumes explicit `null` fields for a stable shape.
