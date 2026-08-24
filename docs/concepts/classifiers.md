# Classifiers

Classifiers help Autofile recognize documents and apply structured updates automatically. They are useful when documents contain predictable text, labels, account numbers, dates, vendor names, or other values that can be detected with rules.

Autofile currently exposes classifiers as ordered classifier blocks. Each block contains YAML rules that describe when the block matches and what actions it should compute.

!!! note "Manual YAML editing"
    Classifier blocks currently require editing a YAML configuration snippet. The syntax is powerful, but it is also exact. See the [classifier rules YAML reference](../reference/classifier-rules-yaml.md) before writing production rules.

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
