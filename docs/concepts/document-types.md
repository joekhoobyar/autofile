# Document Types

A Document Type describes a class of documents and controls which metadata fields are available for documents of that type. Examples include Invoice, Bank Statement, Receipt, Letter, and Contract.

Each document has exactly one Document Type.

## Fields

| Field | Description |
| --- | --- |
| **Slug** | Stable identifier used by classifiers, templates, and by-slug API lookups. |
| **Name** | Label displayed in the UI. |
| **Description** | Optional explanation of the type's purpose. |
| **Metadata Types** | Reusable fields available on documents of this type. |
| **Required Fields** | Selected metadata fields that must contain a non-whitespace value when metadata is saved. |

Slugs may contain lowercase letters, numbers, hyphens, and underscores. A slug cannot be changed after the Document Type is created.

## Create A Document Type

Create the [Metadata Types](metadata-types.md) you need before configuring the Document Type.

1. Open **Document Types** from the main navigation.
2. Select **New Document Type**.
3. Enter a **Slug**, **Name**, and optional **Description**.
4. Select the fields to use from **Metadata Types**.
5. Under **Required Fields**, select each field that must contain a value.
6. Select **Save**.

For example, an Invoice type with the slug `invoice` might use:

| Metadata Type | Required |
| --- | --- |
| Correspondent | Yes |
| Issue Date | Yes |
| Status | No |

Newly selected fields are optional until they are selected under **Required Fields**.

## Required Fields

Required status is configured separately for every Document Type. Correspondent could be required for Invoice documents but optional for Letters.

The document Metadata editor identifies required fields and disables **Save** while any of them are blank.

The initial **Unspecified** Document Type has Correspondent and Issue Date selected and required by default.

!!! note "Required-field scope"
    Required fields are enforced by the Metadata editor and when an individual required value is submitted or deleted. The API does not currently perform a complete required-field check when a document is uploaded, when its type changes, or when a field is newly marked required. Existing documents may therefore need their metadata completed manually.

## Assign A Document Type

Select a Document Type when uploading a document. Autofile selects **Unspecified** by default when that type is available, but the upload request always includes a specific Document Type.

To change an existing document:

1. Open the document.
2. Select **Properties** from the document menu.
3. Select the new **Document Type**.
4. Select **Save**.
5. Open **Metadata** and complete the fields required by the new type.

!!! warning "Changing a Document Type can delete metadata"
    When a document's type changes, Autofile permanently removes stored metadata fields that are not associated with the new type. Fields shared by both types are retained. Review the associations before changing the type.

## Edit Metadata Associations

Edit a Document Type to add or remove its Metadata Types or change which fields are required. Saving replaces the complete set of associations for that Document Type.

Removing an association does not immediately delete existing values from documents of that type. The field disappears from their Metadata editor, but an existing value can remain stored until a later document Properties update cleans it up. Even a title-only Properties update can permanently delete values that are no longer associated with the current type.

Adding a required field does not populate existing documents automatically. Open those documents and enter the new value where needed.

## Delete A Document Type

Deleting a Document Type reassigns its documents to the default Document Type. The default type itself cannot be deleted.

Before deleting a type:

1. Check which documents use it.
2. Confirm the default type is appropriate for those documents.
3. Review metadata associations, because reassignment does not immediately remove all values inherited from the deleted type.

Deleting a Document Type does not queue index updates for its reassigned documents. Rebuild indexes that use `doc.document_type` so they do not retain stale memberships. Reassignment also retains existing metadata; clean up values that are not valid for the default type before rebuilding metadata-based indexes.

## Use Document Type Slugs

Classifier actions select a type by slug:

```yaml
actions:
  _suggested_doctype: invoice
```

Index templates can read the current Document Type slug from `doc.document_type`:

```jinja
{{ doc.document_type }}
```

See [Classifier Blocks](classifier-blocks.md) and [Indexes](indexes.md) for complete workflows.

## Related API

See [Document Types](../reference/api.md#document-types) and [Document Type Metadata Associations](../reference/api.md#document-type-metadata-associations) in the API reference.
