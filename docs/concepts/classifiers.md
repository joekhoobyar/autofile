# Classifiers

Classifiers help Autofile recognize documents and apply structured updates automatically. They are useful when documents contain predictable text, labels, account numbers, dates, vendor names, or other values that can be detected with rules.

Autofile exposes classifiers as ordered classifier blocks. Each block contains structured rules that describe when the block matches and what actions it should compute.

The classifier editor provides visual controls for patterns, metadata conditions, actions, child rules, capture replacements, and modifier pipelines. Advanced YAML mode remains available for direct editing and import. See the [classifier rules YAML reference](../reference/classifier-rules-yaml.md) when reviewing the serialized format.

## What Classifiers Can Do

A classifier can suggest or apply:

- A document type.
- A document title.
- Tags.
- Cabinets.
- Metadata values.

Classifiers can also extract captured text from a document and transform it into useful values, such as normalized dates, padded account numbers, or cleaned currency amounts.

## How to Learn the System

Start with the [classifier block guide](classifier-blocks.md) to understand how blocks run from a user's perspective. Use the [YAML reference](../reference/classifier-rules-yaml.md) when writing or debugging rules.
