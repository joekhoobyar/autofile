# Metadata

Metadata lets Autofile store structured information about documents. It is useful for describing documents consistently, finding them later, applying classifier results, and building [document indexes](indexes.md).

## Metadata Model

Autofile uses three related concepts:

- A **Document Type** describes a class of documents, such as invoices, statements, receipts, letters, or contracts.
- A **Metadata Type** defines a reusable field, such as Correspondent, Issue Date, or Account Number.
- **Document Metadata** is the value stored for one metadata field on one document.

The relationship looks like this:

```text
Document
  └── has one Document Type
        └── selects zero or more Metadata Types
              └── marks each field optional or required

Document + Metadata Type
  └── may store at most one metadata value
```

For example, an Invoice Document Type could use these fields:

| Metadata Type | Data Type | Required | Example Value |
| --- | --- | --- | --- |
| Correspondent | String | Yes | `Acme Corporation` |
| Issue Date | Date | Yes | `2026-08-25` |
| Status | Lookup | No | `Paid` |

The **Required** setting belongs to the relationship between a Document Type and a Metadata Type. The same Metadata Type can be required for one Document Type and optional for another.

## Names And Slugs

Document Types and Metadata Types have both a name and a slug:

- The **name** is the label displayed in the UI, such as `Issue Date`.
- The **slug** is a stable identifier, such as `issue_date`.

Slugs may contain lowercase letters, numbers, hyphens, and underscores. They cannot be changed after creation. Classifiers, index templates, by-slug lookups, and document response metadata maps use slugs. Other REST operations generally identify resources by numeric ID.

For example, an index template reads Issue Date metadata with:

```jinja
{{ doc.metadata["issue_date"] }}
```

## Typical Workflow

1. Create reusable [Metadata Types](metadata-types.md).
2. Create a [Document Type](document-types.md).
3. Select the Metadata Types that apply to that Document Type.
4. Mark fields required where appropriate.
5. Assign the Document Type when uploading or editing a document.
6. Enter values on the document's [Metadata page](document-metadata.md).
7. Use the values in search, classifiers, and indexes.

Autofile initially provides an **Unspecified** Document Type and the **Correspondent** and **Issue Date** Metadata Types. Correspondent and Issue Date are associated with Unspecified and required by default. These resources can be edited and associated with other Document Types.

## Related Features

- [Document Types](document-types.md) explains how to define a document schema and required fields.
- [Metadata Types](metadata-types.md) covers String, Date, and Lookup fields.
- [Document Metadata](document-metadata.md) explains how to enter, validate, and search values.
- [Indexes](indexes.md) explains how metadata slugs organize documents into browsable groups.
- [Classifier Blocks](classifier-blocks.md) can select Document Types and populate metadata automatically.
- The [API reference](../reference/api.md#metadata-resources) documents the related REST resources.
