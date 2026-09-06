# Document Classification

Document classification is the process where by Autofile recognizes documents and apply structured updates automatically. Many documents contain predictable text, labels, account numbers, dates, vendor names, or other values that can be detected with rules - and these rules can be used to automatically classify documents.

Autofile implements document classification with a rules engine.  Rules are grouped into a blocks called classifier blocks.  Each block contains structured rules that describe when the block matches a document and what actions it should take on matched documents.



## What Document Classification Can Do

Document classification can apply:

- A document type.
- A document title.
- Tags.
- Cabinets.
- Document metadata values.

Classifier block rules can also extract captured text from a document and transform it into useful values, such as normalized dates, padded account numbers, or cleaned currency amounts.

## How to Learn the System

The classifier editor provides visual controls for patterns, metadata conditions, actions, child rules, capture replacements, and modifier pipelines. Start with the [classifier block guide](classifier-blocks.md) to understand how blocks run from a user's perspective.

_Edit as YAML_ mode remains available for direct editing and export/import. Advanced users that prefer editing YAML may refer to the [YAML reference](../reference/classifier-rules-yaml.md) when writing or debugging rules.
