# Metadata Types

A Metadata Type defines a reusable field that can be associated with zero or more [Document Types](document-types.md). Examples include Correspondent, Issue Date, Account Number, and Status.

## Fields

| Field | Description |
| --- | --- |
| **Slug** | Stable identifier used by classifiers, templates, and by-slug API lookups. |
| **Name** | Label displayed in the UI. |
| **Data Type** | Determines validation and the editor used for document values. |
| **Description** | Optional explanation of the field. |
| **Choices** | Allowed values for a Lookup field, entered one per line. |

Slugs may contain lowercase letters, numbers, hyphens, and underscores. A slug cannot be changed after the Metadata Type is created.

## Data Types

Autofile supports three metadata data types:

| Data Type | Document Editor | Value Rules | Example |
| --- | --- | --- | --- |
| **String** | Text input | Any text value. | `Acme Corporation` |
| **Date** | Date picker | A valid date stored as `YYYY-MM-DD`. | `2026-08-25` |
| **Lookup** | Dropdown | A configured choice; surrounding whitespace is ignored during validation. | `Paid` |

All document metadata values are represented as strings in the API. The selected data type controls validation and UI behavior.

### String

Use String for names, reference numbers, account numbers, and other free-form text. Autofile does not apply length, format, or uniqueness rules to String values.

### Date

Use Date for calendar dates such as an issue date, statement date, or due date. Values are stored in ISO-style `YYYY-MM-DD` format, which also makes year and month extraction straightforward in [index templates](indexes.md).

For example, `2026-08-25` represents August 25, 2026.

### Lookup

Use Lookup when values should come from a controlled list.

1. Select **Lookup** as the **Data Type**.
2. Enter one allowed value per line under **Choices**.
3. Select **Save**.

For a Status field, choices might be:

```text
Open
Paid
Cancelled
```

Blank lines and surrounding whitespace are removed when choices are saved. Matching is case-sensitive, so `Paid` and `paid` are different values. Submitted values are trimmed for comparison, but the original submitted string is stored.

!!! warning "Lookup fields need choices"
    A Lookup field without choices displays **No choices** in the document editor. A required Lookup field cannot be completed until at least one choice is configured.

## Create A Metadata Type

1. Open **Metadata Types** from the main navigation.
2. Select **New Metadata Type**.
3. Enter a **Slug** and **Name**.
4. Select a **Data Type**.
5. Enter an optional **Description**.
6. For a Lookup field, enter its **Choices**.
7. Select **Save**.

After creating the field, edit a Document Type and select it under **Metadata Types**. Required status is configured on the Document Type, not on the Metadata Type itself.

## Edit A Metadata Type

The name, description, data type, and Lookup choices can be edited. The slug remains unchanged.

!!! warning "Existing values are not migrated"
    Changing a data type or changing Lookup choices does not convert or revalidate existing document values. For example, changing a String field to Date can leave stored text that is not a valid date, and removing a Lookup choice can leave documents with the old value. Review existing documents before changing a field definition.

## Delete A Metadata Type

A Metadata Type can be deleted only when no document metadata values still reference it. Remove or migrate those values first. Deleting an unused Metadata Type also removes its associations with Document Types.

!!! warning "Document values are not deleted automatically"
    Despite the current confirmation-dialog wording, the API does not cascade deletion to existing document metadata. Deletion fails while document values reference the Metadata Type.

## Use Metadata Slugs

Document responses and templates key metadata by slug. For example, the display name **Issue Date** might use the slug `issue_date`:

```json
{
  "metadata": {
    "correspondent": "Acme Corporation",
    "issue_date": "2026-08-25"
  }
}
```

Classifier actions also use the slug:

```yaml
actions:
  correspondent: Acme Corporation
  issue_date: 2026-08-25
```

The Metadata Type must be associated with the document's current Document Type, and the generated value must satisfy its data-type rules.

## Related API

See [Metadata Types](../reference/api.md#metadata-types) in the API reference.
