# Classifier Rules YAML Reference

Classifier block rules are edited with the structured UI and saved as JSON by the API. Advanced YAML mode supports direct editing and import of the same rule structure.

## Top-Level Schema

Every classifier block rule document has this shape:

```yaml
continue_after_match: false
match_patterns: []
match_actions: {}
child_rules: []
```

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `continue_after_match` | boolean | No | Whether classification continues to later blocks after this block matches. Defaults to `false`. |
| `match_patterns` | array | Yes | OR-based top-level patterns that decide whether the block matches. An empty array matches every document. |
| `match_actions` | object | Yes | String key-value actions applied when the block matches. |
| `child_rules` | array | Yes | Child rules evaluated after the block matches. |

## Patterns

A pattern can match document text, metadata, or both.

```yaml
text: "Invoice Number[: ]+([0-9]+)"
metadata:
  vendor: acme
```

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `text` | string | No | Regular expression matched against document text. |
| `metadata` | object | No | String key-value metadata requirements. |

If both `text` and `metadata` are present, both must match.

Empty patterns are allowed. A pattern with no text and no metadata matches every document that reaches it.

## Text Matching

Text patterns are regular expressions. Autofile builds them with case-insensitive and multiline matching enabled.

This pattern matches `invoice`, `Invoice`, or `INVOICE`:

```yaml
text: "invoice"
```

Capture groups are available to child-rule actions and modifiers as numbered snippets.

```yaml
text: "Invoice Number[: ]+([A-Z0-9-]+)"
```

The captured invoice number is snippet `\1`.

## Metadata Matching

Metadata patterns compare exact string values.

```yaml
metadata:
  vendor: acme
  source: scanned-mail
```

All listed key-value pairs must match.

When matching metadata, Autofile checks computed actions first. If a computed action exists for the same key, Autofile compares that value and does not fall back to stored document metadata for that key. If no computed action exists, Autofile checks the document's stored metadata.

## Match Patterns

`match_patterns` is an array of patterns. The block matches if any pattern matches. If the array is empty, the block matches every document.

```yaml
match_patterns:
  - text: "Invoice"
  - metadata:
      document_source: email
```

Use an empty array when a block should apply to every document that reaches it.

## Actions

Actions are string key-value pairs.

```yaml
match_actions:
  _suggested_doctype: invoice
  vendor: acme
```

Child rule actions have the same syntax:

```yaml
actions:
  invoice_number: "\\1"
```

All action values must be strings.

## Special Action Keys

| Action key | Value | Effect |
| --- | --- | --- |
| `_suggested_doctype` | Document type slug | Sets the document type. |
| `_suggested_filename` | Title string | Sets the document title. |
| `_suggested_tags` | Comma-separated tag slugs | Adds the matching tags to the document. |
| `_suggested_cabinets` | Comma-separated cabinet slugs | Adds the document to the matching cabinets. |
| Any key that does not start with `_` | Metadata value | Upserts metadata by metadata type slug. |
| Any other key starting with `_` | Any string | Scratch value. Available to later rules but ignored during persistence. |

Scratch actions are useful for intermediate values that should not be written to the final document:

```yaml
actions:
  _normalized_account: "\\1"
```

Later child rules can match `metadata: { _normalized_account: "123" }`, and the `metadata` modifier can copy `_normalized_account` into a snippet. The structured editor labels these as `Scratch: _normalized_account`.

For comma-separated tag and cabinet lists, whitespace is trimmed and duplicate slugs are ignored.

```yaml
match_actions:
  _suggested_tags: tax, property, tax
  _suggested_cabinets: household-records
```

## Child Rules

Child rules run only after the parent block matches.

```yaml
child_rules:
  - pattern:
      text: "Invoice Number[: ]+([A-Z0-9-]+)"
    modifiers:
      - type: zero_pad
        from: "\\1"
        to: 2
        length: 8
    actions:
      invoice_number: "\\2"
```

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `pattern` | object | Yes | Pattern that decides whether this child rule applies. |
| `modifiers` | array | No | Transformations that create or update snippets. |
| `actions` | object | Yes | String key-value actions applied when the child rule matches. |

Autofile evaluates every child rule in order. Every matching child rule applies its actions.

## Snippet Replacement

When a child rule text pattern matches, capture groups become snippets:

- `\1`: first capture group.
- `\2`: second capture group.
- `\3`: third capture group.

Actions and modifier `from` values can use snippets.

```yaml
actions:
  invoice_number: "INV-\\1"
```

If a referenced snippet does not exist, it is replaced with an empty string.

Snippet replacements are not regular-expression backreferences. Use capture groups such as `([0-9]+)` in the child pattern, then insert `\1` in child action values or string-valued modifier `from` fields. Do not put `\1` in the regular expression itself; Rust regular expressions do not support backreferences.

The structured editor displays available captures and modifier outputs as insertable `\N` buttons. A button inserts the replacement at the current cursor position.

YAML escaping matters:

| YAML style | Write snippet as |
| --- | --- |
| Double quoted | `"\\1"` |
| Single quoted | `'\1'` |
| Plain scalar | `\1` |

Double-quoted strings are common in examples because they work well with regular expression punctuation, but they require escaping backslashes.

## Modifiers

Modifiers transform snippets. Each modifier has a `type` and writes its result to the snippet index named by `to`.

Modifiers run in order. Later modifiers can use snippets created by earlier modifiers.

If a modifier fails, Autofile logs a warning and continues. The failed modifier does not write its output snippet.

### `metadata`

Copies a computed action into a snippet.

```yaml
- type: metadata
  slug: invoice_number
  to: 2
```

| Field | Type | Description |
| --- | --- | --- |
| `slug` | string | Computed action key to read. |
| `to` | number | Snippet index to write. |

### `month_number`

Converts a month name or abbreviation to a two-digit month number.

```yaml
- type: month_number
  from: "\\1"
  to: 2
```

Examples:

| Input | Output |
| --- | --- |
| `January` | `01` |
| `sep` | `09` |

### `month_start`

Converts a `YYYY-MM-DD` date to the first day of that month.

```yaml
- type: month_start
  from: "\\1"
  to: 2
```

Example: `2024-02-10` becomes `2024-02-01`.

### `month_end`

Converts a `YYYY-MM-DD` date to the last day of that month.

```yaml
- type: month_end
  from: "\\1"
  to: 2
```

Example: `2024-02-10` becomes `2024-02-29`.

### `next_day`

Adds days to a `YYYY-MM-DD` date.

```yaml
- type: next_day
  from: "\\1"
  to: 2
```

By default, it adds one day. To add a specific number of days, use `days|date`:

```yaml
- type: next_day
  from: "2|\\1"
  to: 2
```

Examples:

| Input | Output |
| --- | --- |
| `2024-01-10` | `2024-01-11` |
| `2|2024-01-10` | `2024-01-12` |

### `prev_day`

Subtracts days from a `YYYY-MM-DD` date.

```yaml
- type: prev_day
  from: "\\1"
  to: 2
```

By default, it subtracts one day. To subtract a specific number of days, use `days|date`.

Example: `2|2024-01-10` becomes `2024-01-08`.

### `next_month`

Adds months to a `YYYY-MM-DD` date.

```yaml
- type: next_month
  from: "\\1"
  to: 2
```

By default, it adds one month. To add a specific number of months, use `months|date`.

If the target month has fewer days, the result clamps to the last valid day of the target month. Example: `2024-01-31` becomes `2024-02-29`.

### `prev_month`

Subtracts months from a `YYYY-MM-DD` date.

```yaml
- type: prev_month
  from: "\\1"
  to: 2
```

By default, it subtracts one month. To subtract a specific number of months, use `months|date`.

If the target month has fewer days, the result clamps to the last valid day of the target month. Example: `2024-03-31` becomes `2024-02-29`.

### `tax_year`

Adds one month to a `YYYY-MM-DD` date and returns the resulting year.

```yaml
- type: tax_year
  from: "\\1"
  to: 2
```

Example: `2024-12-31` becomes `2025`.

### `currency`

Normalizes currency text by removing `$`, commas, and leading zeroes before the first non-zero digit.

```yaml
- type: currency
  from: "\\1"
  to: 2
```

Example: `$001,234` becomes `1234`.

### `zero_pad`

Left-pads a value with zeroes until it reaches the requested length.

```yaml
- type: zero_pad
  from: "\\1"
  to: 2
  length: 4
```

Examples:

| Input | Length | Output |
| --- | --- | --- |
| `7` | `4` | `0007` |
| `1234` | `4` | `1234` |
| `12345` | `4` | `12345` |

### `replace`

Builds a new snippet by applying snippet replacement to `from`.

```yaml
- type: replace
  from: "INV-\\1"
  to: 2
```

Example: if `\1` is `123`, output is `INV-123`.

### `alnum_sanitize`

Keeps ASCII letters and numbers, preserves normalized spaces, and removes other punctuation.

```yaml
- type: alnum_sanitize
  from: "\\1"
  to: 2
```

Example: ` ACME-123 / West ` becomes `ACME123 West`.

### `date_format`

Formats a `YYYY-MM-DD` date using a chrono format string.

```yaml
- type: date_format
  from: "\\1"
  to: 2
  format: "%m/%d/%Y"
```

Example: `2024-01-10` becomes `01/10/2024`.

### Arithmetic Modifiers

Arithmetic modifiers operate on numeric snippets. They read snippet `from`, combine it with snippet `to`, and write the result back to snippet `to`.

Values are parsed as numbers after trimming commas and `$`.

| Type | Operation |
| --- | --- |
| `add` | `to = to + from` |
| `sub` | `to = to - from` |
| `mul` | `to = to * from` |
| `div` | `to = to / from` |

```yaml
- type: add
  from: 1
  to: 2
```

Example with snippets `\1 = 2` and `\2 = 10`: `add` writes `12` to `\2`.

Division by zero fails and leaves the destination snippet unchanged.

## Full Example

```yaml
continue_after_match: false
match_patterns:
  - text: "Invoice"
match_actions:
  _suggested_doctype: invoice
  _suggested_tags: finance,accounts-payable
child_rules:
  - pattern:
      text: "Invoice Number[: ]+([A-Z0-9-]+)"
    actions:
      invoice_number: "\\1"
  - pattern:
      text: "Invoice Date[: ]+([0-9]{4}-[0-9]{2}-[0-9]{2})"
    modifiers:
      - type: tax_year
        from: "\\1"
        to: 2
    actions:
      invoice_date: "\\1"
      tax_year: "\\2"
  - pattern:
      text: "Total[: ]+([$0-9,]+(?:\\.[0-9]{2})?)"
    modifiers:
      - type: currency
        from: "\\1"
        to: 2
    actions:
      invoice_total: "\\2"
```
