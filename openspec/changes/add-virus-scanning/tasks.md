## 1. Data Model And Settings

- [x] Add a migration for `app_settings.virus_scanning_enabled` and `app_settings.virus_scan_by_default`.
- [x] Add a migration for document-file scan fields and scan-status constraints.
- [x] Regenerate `api/src/schema.rs`.
- [x] Update API domain models and UI models for the new settings, scan fields, and derived `content_available` field.

## 2. Scanner Infrastructure

- [x] Add scanner configuration parsing and startup validation.
- [x] Add a `MalwareScanner` abstraction and scanner outcome types.
- [x] Implement the ClamAV `clamd` TCP `INSTREAM` provider.
- [x] Add fake scanner support for unit/integration tests.

## 3. Upload And Job Flow

- [x] Update document and document-file upload endpoints to accept a per-upload virus-scan field.
- [x] Resolve per-upload scan intent from app settings and multipart input.
- [x] Insert uploaded files with `pending` or `not_required` scan status.
- [x] Add a scan background job that streams the S3 object to the scanner.
- [x] Enqueue normal post-upload processing only after files are clean or not required.
- [x] Preserve existing S3 cleanup behavior on transaction/job-enqueue failure.

## 4. Processing And Download Gates

- [ ] Add a shared helper for determining whether document-file content is available for use and for producing blocked-file messages.
- [ ] Block downloads and download tickets for pending, scanning, infected, or errored files.
- [ ] Block previews, page images, extracted text, OCR text, thumbnail generation/retrieval, page processing, OCR, classification, and all other file-content operations until files have `content_available=true`.
- [ ] Ensure deletion and explicit rescan/retry-scan operations are allowed for files with `content_available=false`.
- [ ] Return clear conflict errors for manual processing endpoints blocked by scan status.
- [ ] Add API tests that verify unsafe files are refused by download, page-image, text, OCR, thumbnail, processing, and classification paths.
- [ ] Add API tests that verify unsafe files can still be deleted and can be submitted for explicit rescan/retry.

## 5. UI

- [ ] Update settings page controls for virus scanning and scan-by-default.
- [ ] Update upload pages with a `Virus scan this upload` checkbox shown only when scanning is enabled.
- [ ] Update document properties/file metadata views to display scan status per file.
- [ ] Add a document properties/file metadata action for submitting eligible files for rescan/retry-scan when virus scanning is enabled.
- [ ] Update file preview/page/text/OCR/download surfaces to display unavailable-file messages when `content_available=false`.
- [ ] Add UI tests for settings visibility, upload checkbox defaults, status display, rescan/retry-scan action visibility/state, blocked-file messages, and normal rendering when `content_available=true`.

## 6. Deployment And Documentation

- [ ] Add a Docker Compose `clamav` service using the ClamAV container image and API environment examples for local scanner testing.
- [ ] Add the Wiremind ClamAV chart as a Helm dependency gated by `clamav.enabled` and update `Chart.lock`.
- [ ] Add Helm values/templates using `virusScanning.provider`, `virusScanning.clamavHost`, `virusScanning.clamavPort`, and `clamav.enabled` to configure bundled Wiremind ClamAV, external ClamAV, and disabled scanner modes without maintaining first-party ClamAV Deployment/Service templates.
- [ ] Update README configuration table.
- [ ] Update docs administration/configuration and background-job documentation.
- [ ] Update API reference for new settings, upload field, and response fields.

## 7. Verification

- [ ] Run `openspec validate add-virus-scanning --strict`.
- [ ] Run relevant API checks from `api/`.
- [ ] Run relevant UI checks from `ui/`.
- [ ] Run Compose and Helm rendering checks for deployment changes.
