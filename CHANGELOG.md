# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.4.5] - 2026-09-08

### Added

- Automate cutting and reopening CHANGELOG.md release sections in `make release`.
- Add read-only detail pages for document types and metadata types, including linked metadata type chips and usage indicators.
- Add an authenticated `GET /api/v1/ping` endpoint for checking whether credentials are still valid.

### Changed

- Restrict document type, metadata type, and document type metadata association administration to admin users while preserving read access for regular users.

### Fixed

- Prevent users from deleting their own account in both the API and UI.
- Check authentication with `GET /api/v1/ping` before document and document-file uploads so expired access tokens can refresh before file transfer starts.

## [0.4.4] - 2026-09-07

### Changed

- Initial documented release notes for automated GitHub releases.
- Disable metadata type deletion when the type has document metadata values.

## [0.4.3] - 2026-09-07

### Changed

- Prevent deleting metadata types in-use by a document type
