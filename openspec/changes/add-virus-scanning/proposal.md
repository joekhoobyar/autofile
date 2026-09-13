## Why

Autofile accepts untrusted document uploads and then processes those files with PDF, image, OCR, office-document, and text-extraction tools. Malware scanning should happen before any file is downloaded by users or processed by background jobs so Autofile protects both users and the document-processing infrastructure.

The feature needs to fit Autofile's self-hosted deployment model and S3-compatible storage abstraction. ClamAV `clamd` with the `INSTREAM` protocol is the initial provider because it is free, self-hostable, widely understood, and can scan bytes streamed from S3 without giving the scanner direct object-storage credentials.

## What Changes

- Add application settings for enabling virus scanning and choosing whether new uploads are scanned by default.
- Add a per-upload checkbox in the document upload UI when virus scanning is enabled.
- Store uploaded file bytes durably in S3 immediately, but mark the associated document file as unavailable until scanning is complete when a scan is requested.
- Add scan status and scan metadata to document files.
- Add an asynchronous scanning job that streams the stored object to the configured scanner and updates the document file status.
- Prevent file downloads, previews, thumbnail generation, page extraction, OCR, classification, and index updates from using files that still require a clean scan.
- Expose a derived `content_available` value so UI and backend callers do not duplicate scan-status checks everywhere.
- Display scan status and blocked-file messages on document properties and file-oriented display pages.
- Add a ClamAV container to Docker Compose for local testing, plus Helm configuration that bundles the Wiremind ClamAV chart when `clamav.enabled=true`.

## Capabilities

### New Capabilities

- `virus-scanning`: Defines configurable malware scanning for uploaded document files, scan-status tracking, processing/download gates, and initial ClamAV integration.

### Modified Capabilities

- Document upload and document-file upload behavior.
- Document file metadata shown in the UI.
- Application settings management.
- Deployment configuration for Compose and Helm.

## Impact

- Adds migrations for app settings and document-file scan fields.
- Adds scanner configuration parsing and shared scanner infrastructure in the API.
- Adds a new background job variant for scanning uploaded document files.
- Updates document and document-file creation workflows.
- Updates download, preview, and processing workflows to require a clean scan when scanning was requested.
- Updates UI models, queries, settings page, upload forms, document properties display, and blocked-file messages.
- Updates docs, Docker Compose, and Helm chart values/templates.
