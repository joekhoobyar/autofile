# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed

- Fixed openapi job in release workflow

## [0.5.1] - 2026-09-12

### Fixed

- Fixed openapi job in release workflow
- Fixed attestation job in release workflow

## [0.5.0] - 2026-09-12

### Changed

- Implemented OpenAPI for all routes

### Security

- Fix uncontrolled allocation size (#33)

## [0.4.14] - 2026-09-12

### Changed

- helm / docker compose: Update rustfs chart to 1.0.0-rc6
- helm: Update valkey chart to 0.12.0

## [0.4.13] - 2026-09-12

### Added

- Add chart metadata for artifacthub.io

## [0.4.12] - 2026-09-11

### Fixed

- Keep cabinet, tag, and document index scopes applied when searching documents, and clarify advanced document search grouping.

## [0.4.11] - 2026-09-11

### Changed

- Adjust the layout of the quick filter controls on the documents list page.
- Adjust the paginator on the documents list page
- Soft-delete users, hide deleted users by default, and support restoring deleted users from administration.
- Re-registering with a deleted user's email now restores that user when the requested username is available.

## [0.4.10] - 2026-09-11

### Added

- Add autocomplete for metadata value editing.
- Add a public sign-up page linked from the login page, with a success message directing new users to wait for an administrator to enable their account.
- Add an unauthenticated `GET /api/v1/public/settings` endpoint exposing the user-registration flag so the UI can show or hide sign-up accordingly.

### Fixed

- Blank metadata values are now properly deleted from the document.

## [0.4.9] - 2026-09-10

### Changed

- Adjust the layout of the quick filter controls on the documents list page.
- Migrated documentation to Zensical

### Fixed

- Refresh tag, cabinet, and document type document counts after adding/removing documents to/from tags or cabinets, changing a document's type, or deleting a document.

## [0.4.8] - 2026-09-10

### Added

- Show document counts in the advanced search tag, cabinet, and document type dropdowns, with counts right-aligned.
- Add compact Tags, Cabinets, and Document Type quick-filter dropdowns to the main documents list header, after the Advanced Search link.
- Add a document count column to the document types list page and include `document_count` in `GET /api/v1/document-types` list responses.

### Changed

- Use a hierarchical cabinet tree selector on advanced search, showing unjoined cabinet names with document counts.
- Adjust the layout of the controls on the documents list page.

## [0.4.7] - 2026-09-09

### Added

- Support adding scratch values as classifier pattern metadata conditions.
- Add **Download** to Actions menu, supporting multi-file downloads.

### Changed

- Changed to bootstrap dark blue theme.

## [0.4.6] - 2026-09-08

### Added

- Add an admin-only Settings page and `/api/v1/settings` API for controlling app behavior, starting with an `allow_user_registration` toggle that blocks `POST /api/v1/auth/register` when disabled.
- Add user enabled/disabled status, require admins to enable newly registered users, and block login or token refresh for disabled users.

### Changed

- Improve read-only detail page presentation with a shared responsive description-list layout for users, document types, metadata types, and about information.
- Upgrade background job processing to Apalis 1.0 (`apalis` 1.0.0-rc.9, `apalis-redis` 1.0.0-rc.8) and Redis client 1.x, migrating worker registration to the 1.0 factory-based APIs while keeping job handlers on the project-owned `JobResult` error type.

### Fixed

- Cancel document file page processing when the document file is deleted mid-job to avoid repeated retry noise.
- Cancel thumbnail generation when the document file is deleted mid-job to avoid repeated retry noise.

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
