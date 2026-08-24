# Classifier Blocks

A classifier block is an ordered rule set that tries to recognize a document and compute actions for it. Actions can set the document type, title, tags, cabinets, or metadata.

Classifier blocks are powerful because each block can do two things:

- Decide whether the block applies to a document.
- Extract or compute values through child rules after the block applies.

## When Classifier Blocks Run

Autofile runs classifier blocks during document classification. Classification can be triggered after document processing or manually from a document action.

When classification starts, Autofile loads:

- The document record and existing document metadata.
- The document text, using extracted text first and OCR text as a fallback.
- All enabled classifier blocks.

Disabled blocks are ignored.

## Execution Order

Classifier blocks run in ascending order by their `order` field. If two blocks have the same order, Autofile uses the block `id` as a tie-breaker.

The Classifiers page shows the block order and allows reordering when the list is sorted by order ascending.

Order matters because:

- Earlier blocks run first.
- Earlier blocks can compute actions that later blocks use for metadata matching.
- Later actions overwrite earlier actions with the same key.
- A matching block can stop the entire classification run unless `continue_after_match` is enabled.

## Block Matching Flow

For each enabled classifier block, Autofile evaluates the block's `match_patterns`.

```yaml
continue_after_match: false
match_patterns:
  - text: "invoice"
match_actions:
  _suggested_doctype: invoice
child_rules: []
```

A block matches when any top-level pattern in `match_patterns` matches the document. If `match_patterns` is empty, the block matches every document.

When a block matches, Autofile:

- Applies the block's `match_actions`.
- Evaluates every child rule in `child_rules`, in order.
- Applies actions from every matching child rule.
- Stops classification if `continue_after_match` is `false`.
- Continues to the next enabled block if `continue_after_match` is `true`.

When a block does not match, Autofile skips its `match_actions` and `child_rules`, then continues to the next block.

## Continue Or Stop

Most classifier blocks should use the default behavior:

```yaml
continue_after_match: false
```

This means “when this block matches, apply its actions and stop looking for more blocks.” It is useful when a block fully classifies a document.

Use this when the block should feed later blocks:

```yaml
continue_after_match: true
```

This means “when this block matches, apply its actions and keep going.” It is useful for layered classification, such as first detecting a vendor and then letting later vendor-specific blocks extract dates or account numbers.

## Actions

Actions are string key-value pairs. Some action keys have special meanings.

| Action key | Effect |
| --- | --- |
| `_suggested_doctype` | Sets the document type by document type slug. |
| `_suggested_filename` | Sets the document title. |
| `_suggested_tags` | Adds tags by comma-separated tag slugs. |
| `_suggested_cabinets` | Adds cabinets by comma-separated cabinet slugs. |
| Any key that does not start with `_` | Upserts document metadata by metadata type slug. |
| Unknown key starting with `_` | Ignored. |

The referenced document type, tag, cabinet, and metadata type slugs must already exist. Unknown tag and cabinet slugs are skipped. Unknown metadata slugs are logged and skipped.

## Computed Actions

Autofile stores intermediate classification results in a map called computed actions. The map is built during classification and persisted only after the classification flow completes.

Computed actions are important because:

- `match_actions` write into computed actions.
- Matching child rules write into computed actions.
- Later actions with the same key overwrite earlier values.
- Metadata patterns check computed actions before stored document metadata.
- The `metadata` modifier can copy a computed action into a child-rule snippet.

## Top-Level Patterns

Top-level patterns decide whether a block applies.

```yaml
match_patterns:
  - text: "Invoice Number"
  - metadata:
      source: scanned-mail
```

The block matches if any listed pattern matches. A pattern can contain text, metadata, or both.

Text patterns are regular expressions matched against the document text. Matching is case-insensitive and multiline.

Metadata patterns require all listed metadata values to match exactly. Metadata matching checks computed actions first, then existing document metadata.

## Child Rules

Child rules run only after the parent block matches. Each child rule has its own pattern and actions.

```yaml
child_rules:
  - pattern:
      text: "Invoice Number[: ]+([A-Z0-9-]+)"
    actions:
      invoice_number: "\\1"
```

Autofile evaluates every child rule in order. Child rules do not stop after the first match. Every matching child rule applies its actions.

Use child rules to extract specific values after a document has already been recognized by the parent block.

## Captures And Snippets

When a child rule text pattern uses regular expression capture groups, those captures become snippets.

For this pattern:

```yaml
text: "Invoice Number[: ]+([A-Z0-9-]+)"
```

The first capture group is available as `\1`. Actions can insert snippets into values:

```yaml
actions:
  invoice_number: "\\1"
```

In YAML double-quoted strings, write `\1` as `"\\1"`. In YAML single-quoted strings, write it as `'\1'`.

Modifiers can create additional snippets. For example, this rule captures `7`, pads it to four digits, and stores `0007`:

```yaml
child_rules:
  - pattern:
      text: "Account[: ]+([0-9]+)"
    modifiers:
      - type: zero_pad
        from: "\\1"
        to: 2
        length: 4
    actions:
      account_number: "\\2"
```

## Example: Simple Document Type

This block recognizes an invoice and sets its document type.

```yaml
continue_after_match: false
match_patterns:
  - text: "invoice"
match_actions:
  _suggested_doctype: invoice
child_rules: []
```

## Example: Extract An Invoice Number

```yaml
continue_after_match: false
match_patterns:
  - text: "invoice"
match_actions:
  _suggested_doctype: invoice
child_rules:
  - pattern:
      text: "Invoice Number[: ]+([A-Z0-9-]+)"
    actions:
      invoice_number: "\\1"
```

## Example: Layered Classification

This first block identifies the vendor and allows classification to continue.

```yaml
continue_after_match: true
match_patterns:
  - text: "ACME Corporation"
match_actions:
  vendor: acme
child_rules: []
```

A later block can use that computed vendor value.

```yaml
continue_after_match: false
match_patterns:
  - metadata:
      vendor: acme
match_actions:
  _suggested_tags: vendor-acme
child_rules: []
```

## Example: Dates And Periods

This block extracts a statement date and computes the beginning and end of that month.

```yaml
continue_after_match: false
match_patterns:
  - text: "Statement"
match_actions:
  _suggested_doctype: statement
child_rules:
  - pattern:
      text: "Statement Date[: ]+([0-9]{4}-[0-9]{2}-[0-9]{2})"
    modifiers:
      - type: month_start
        from: "\\1"
        to: 2
      - type: month_end
        from: "\\1"
        to: 3
    actions:
      statement_date: "\\1"
      statement_period_start: "\\2"
      statement_period_end: "\\3"
```

## Testing A Classifier Block

Open a document and use the Classifier Test page to test a single classifier block against that document.

The test result shows computed actions as YAML. This is useful for confirming that a block matches and that child rules produce the expected values.

!!! warning "Single-block tests"
    Testing evaluates the selected classifier block by itself. It does not simulate earlier blocks unless the selected block computes the same actions internally.

## Writing Safe Rules

- Start with a disabled block while drafting.
- Test against documents that should match and documents that should not match.
- Prefer specific top-level match patterns so a block does not classify unrelated documents.
- Use `continue_after_match: true` only when later blocks intentionally depend on earlier computed actions.
- Keep metadata, tag, cabinet, and document type slugs consistent with existing records.
- Use the [classifier rules YAML reference](../reference/classifier-rules-yaml.md) for exact syntax.
