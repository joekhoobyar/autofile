# Document Metadata

Document metadata is the structured information stored for a specific document. A document's current [Document Type](document-types.md) determines which [Metadata Types](metadata-types.md) appear in its Metadata editor.

For example:

```text
Document: August Invoice
Document Type: Invoice
Correspondent: Acme Corporation
Issue Date: 2026-08-25
Status: Paid
```

## Edit Document Metadata

1. Open a document.
2. Select **Metadata** from the document menu.
3. Select a cell in the **Value** column.
4. Enter or select the value.
5. Complete every field marked **Required**.
6. Select **Save**.

Fields are displayed alphabetically by name. The value editor depends on the Metadata Type:

| Data Type | Editor | Behavior |
| --- | --- | --- |
| String | Text input | Accepts free-form text. |
| Date | Date picker | Stores a valid `YYYY-MM-DD` date. |
| Lookup | Dropdown | Selects one of the configured choices. |

The editor sends only values that changed. Existing values for other fields remain unchanged.

## Required Values

The **Required** column shows whether the field is required for the document's current Document Type. If required values are missing, Autofile displays a warning such as:

```text
Required fields are missing: Correspondent, Issue Date
```

The Metadata editor disables **Save** until those fields contain non-whitespace values.

!!! note "Required-field scope"
    A document can currently be uploaded or changed to a new Document Type before its required metadata is complete. The API validates required fields that are submitted, but does not check every required field on every document operation. Use the Metadata editor after upload or a type change to complete the document.

## Validation

The API validates every submitted value against the document's current Document Type:

- The Metadata Type must be associated with the Document Type.
- A submitted required value cannot be blank.
- A Date must be a real calendar date in `YYYY-MM-DD` format.
- A Lookup value must match one of the configured choices after surrounding whitespace is trimmed for validation.
- String values have no additional format restrictions.

Avoid leading or trailing whitespace. Validation ignores surrounding whitespace, but the submitted text is stored as provided.

## Empty And Deleted Values

Clearing an optional value in the UI stores an empty string. It does not remove the metadata record. This distinction matters when searching for documents that have a particular Metadata Type.

The API provides a separate delete operation for a metadata value. Required values cannot be deleted while the field remains required for the current Document Type.

## Change The Document Type

Changing a document's type affects its available metadata:

- Values associated with both the old and new types are retained.
- Values not associated with the new type are permanently deleted.
- Newly available fields start empty unless a value already exists.
- Newly required fields are not populated automatically.

See [Assign A Document Type](document-types.md#assign-a-document-type) before changing an existing document.

## Search Metadata

The basic document search checks title, extracted document text, and metadata values. Metadata matching is case-insensitive and finds partial values.

Use **Advanced Document Search** for more control:

1. Enter a **Metadata Value**.
2. Optionally select a **Metadata Type** to restrict the value to one field.
3. Optionally select a **Document Type**.
4. Select **Match Any** to match any criterion instead of requiring all criteria.
5. Select **Search**.

Selecting a Metadata Type without entering a value finds documents that have a stored record for that field. An optional field stored as an empty string still counts as present.

## Use Metadata In Indexes

Index templates access values by Metadata Type slug:

```jinja
{{ doc.metadata["correspondent"] }}
```

For a Date value, slices can create year and month groups:

```jinja
{{ doc.metadata["issue_date"][:4] if doc.metadata["issue_date"] is defined }}
{{ doc.metadata["issue_date"][5:7] if doc.metadata["issue_date"] is defined }}
```

Saving or deleting metadata queues index updates for that document. See [Indexes](indexes.md) for Correspondent, Issue Date, and combined examples.

## Apply Metadata With Classifiers

Classifier actions can populate metadata by slug:

```yaml
actions:
  _suggested_doctype: invoice
  correspondent: Acme Corporation
  issue_date: 2026-08-25
```

The Metadata Types must be associated with the resulting Document Type, and Date and Lookup values must pass the same validation as values entered in the UI. An unknown metadata slug is skipped, but an invalid value for a known field fails classification. Classifier persistence is not atomic, so title, type, tag, or cabinet changes may already have been saved when later metadata validation fails. See [Classifier Blocks](classifier-blocks.md) for the full rule format.

## Troubleshooting

If a field or value does not behave as expected:

- Confirm the Metadata Type is associated with the document's current Document Type.
- Confirm required fields contain more than whitespace.
- Use `YYYY-MM-DD` for Date values.
- Confirm a Lookup value matches a configured choice after surrounding whitespace is trimmed, including capitalization.
- Review the document's type if a field disappeared after editing Properties.
- Use Metadata Type slugs, not display names, in classifier actions and templates. Document metadata API writes use numeric Metadata Type IDs.
- Rebuild an index after changing its templates; metadata changes update enabled indexes automatically.

## Related API

See [Document Metadata](../reference/api.md#document-metadata) in the API reference.
