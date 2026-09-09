# Document Classification

Document classification is the process Autofile uses to recognize documents and automatically apply structured updates to them.

Many documents contain predictable text, such as account numbers, dates, vendor names, and other identifiable values. Autofile can detect these values using rules and use the results to classify documents automatically.

Autofile's classification system is built around a rules engine. Rules are organized into [classifier blocks](classifier-blocks.md). Each classifier block contains a structured set of rules that defines when the block matches a document and which actions Autofile should perform when a match occurs.

## What Can Document Classification Do?

Document classification can match document text, document metadata, or both to determine which documents should be classified. It can also capture text from a document and transform it into useful values, such as normalized dates, account numbers, or cleaned currency amounts.

When a document matches, classification can automatically apply or update:

- Document type
- Document title
- Tags
- Cabinets
- Document metadata values

It can even integrate previously captured text into the above values.

## When Does Document Classification Run?

Document classification runs automatically when a document with a document type of **Unspecified** has been uploaded and all of it's text contents have been processed (including OCR).

Document classification can also be run manually by choosing **Classify Document** from the document actions menu.

## How to Learn the System

The classifier editor provides visual controls for defining patterns, metadata conditions, actions, child rules, capture replacements, and modifier pipelines. Start with the [classifier block guide](classifier-blocks.md) for an overview of how classifier blocks work from a user's perspective.

For users who prefer to work directly with YAML, *Edit as YAML* mode is available for editing, importing, and exporting rules. The [YAML reference](../reference/classifier-rules-yaml.md) provides detailed syntax and structure for writing or troubleshooting classifier rules.
