![Autofile logo](assets/favicon.png){ align=left width="180" style="margin: 0 1.25rem 1rem 0;" }

# Autofile Documentation

Autofile is a self-hosted document management application. It stores document files in S3-compatible object storage, keeps structured metadata in PostgreSQL, and provides a React web UI backed by a Rust API.

!!! warning Alpha software
    Autofile is relatively stable, but it is still alpha software. We will avoid breaking changes to APIs whenever possible. Expect installation instructions, administration, and documentation to continue to mature.

![document grid basic](assets/document-grid-basic.png){ align=left width="100%" style="margin: 0 0 1rem 0;"  class="skip-lightbox" }

## Features

- A completely self-hosted [document](concepts/documents.md) management solution.
- Performs OCR on documents, making everything searchable - even images.
- Supports PDF documents, images, plain text files, Office documents (Word, Excel, PowerPoint, and LibreOffice equivalents) and more.
- Extracts high-resolution images for all document pages, for quick viewing in the browser.
- [Organizes](concepts/organization.md) documents with cabinets, tags and metadata.
- Assigns [metadata](concepts/metadata.md) to documents.
- Automatically [classify](concepts/classification.md) documents to assign tags, metadata, document types, and organize into cabinets.
- Filter by tag, metadata, document type, cabinets, and more.
- Full text search.
- Automatically builds document [indexes](concepts/indexes.md) with dynamic templates (using [Minijinja](https://docs.rs/minijinja/latest/minijinja/syntax/)).
- Stores files in durable object storage (i.e.: [RustFS](https://rustfs.com/), [Garage](https://garagehq.deuxfleurs.fr/), or [SeaweedFS](https://seaweedfs.com/)).

## Quick Links

- [Installation](getting-started/installation.md)
- [Helm Installation](getting-started/helm.md)
- [Docker Compose Quick Start](getting-started/quick-start.md)
- [Default Admin User](getting-started/first-user.md)
- [Metadata](concepts/metadata.md)
- [Indexes](concepts/indexes.md)
- [Configuration](admin/configuration.md)
- [Development](development/index.md)
- [API Reference](reference/api.md)

## Architecture

Autofile is built as two containers:

- `autofile-api`: Rust/Axum API, background workers, PostgreSQL migrations, S3 file storage, and document processing tools.
- `autofile-ui`: React/Vite frontend served by nginx.

The supporting services are PostgreSQL, Redis, and S3-compatible object storage.
