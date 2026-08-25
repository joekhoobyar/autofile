# Indexes

Document indexes organize documents using derived or templated values.

## Document Indexes

A document index defines an indexable grouping or lookup strategy for documents. Indexes can help build structured views over a collection of documents.

## Index Templates

Index templates define how index values are generated. Templates can reference document data and metadata to produce consistent index values.

!!! note "Template editing"
    Index templates require editing a Minijinja configuration snippet. The syntax is powerful, but it is also exact. See the [Minijinja Syntax reference](https://docs.rs/minijinja/latest/minijinja/syntax/) before writing templates.

## Index Values

Index values are the values assigned to documents for a given index. They are useful when documents need to be grouped, ordered, or found by structured keys.
