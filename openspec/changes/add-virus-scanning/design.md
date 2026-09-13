## Overview

Virus scanning should protect Autofile before uploaded files are parsed, converted, thumbnailed, OCRed, classified, indexed, or served back to users. The initial implementation should use a small scanner abstraction with a ClamAV `clamd` provider, but the ingestion model should not make ClamAV a permanent architectural dependency.

The upload flow must not depend on restart-sensitive local temporary storage after the request completes. Uploaded bytes should be written to S3-compatible object storage once, using the normal document-file object key, and the database row should control whether that object is available for downstream use. When a scan is requested, the file is durable immediately but unavailable until the asynchronous scan job marks it clean.

This design intentionally avoids a separate S3 quarantine prefix and promotion copy. A separate prefix would make access-control semantics visually obvious, but promotion requires copying the full object after scan. Autofile can get the same safety property by treating `document_files.scan_status` as the stored authority and exposing a derived `content_available` boolean everywhere code or UI needs the simpler question: may Autofile read this file's bytes?

## Goals

- Scan untrusted uploaded files before any user download or document-processing job consumes them.
- Keep uploads durable after the HTTP request returns, even if the API restarts before scanning completes.
- Avoid copying S3 objects from a quarantine prefix into a final prefix after scan.
- Make virus scanning explicitly configurable through application settings.
- Let users opt in or out per upload when scanning is enabled, with an administrator-controlled default.
- Show scan status and unavailable-file messages on document/file-oriented pages so users can understand why a file is not yet processed, previewable, or downloadable.
- Keep the initial provider self-hosted and simple: ClamAV `clamd` over TCP using `INSTREAM`.
- Preserve a narrow provider abstraction for future ICAP, MetaDefender, ESET, AWS GuardDuty, or YARA-related work.

## Non-Goals

- No ICAP, MetaDefender, ESET, AWS GuardDuty, VirusTotal, or YARA implementation in the first change.
- No rescan-after-signature-update workflow in the first change.
- No per-user or per-document-type scan policy in the first change.
- No attempt to use ClamAV as a content-disarm-and-reconstruction system.
- No guarantee that existing files uploaded before the feature are retroactively scanned.

## Settings

Autofile already stores mutable application settings in the single-row `app_settings` table. Add two fields there:

- `virus_scanning_enabled boolean not null default false`
- `virus_scan_by_default boolean not null default true`

`virus_scanning_enabled` controls whether the feature is active at the application level. If false, the API skips scan jobs and the UI hides virus-scanning controls and status-specific UI.

`virus_scan_by_default` is only meaningful when `virus_scanning_enabled` is true. It controls the default checked state for upload forms. Users may override it per upload through a checkbox labeled `Virus scan this upload`.

The authenticated settings API should expose both fields to administrators. The public settings API should expose both fields because upload pages need them client-side and the values do not reveal secrets. The settings page should render `virus_scan_by_default` only when `virus_scanning_enabled` is checked.

Scanner connection details should remain environment/deployment configuration rather than database settings because they are operational secrets/topology rather than product behavior:

- `MALWARE_SCANNER_PROVIDER`: `clamav`, `external`, or unset/`disabled`; default `disabled`.
- `CLAMAV_HOST`: default `clamav` when provider is `clamav`.
- `CLAMAV_PORT`: default `3310`.
- `CLAMAV_CONNECT_TIMEOUT_SECONDS`: default `5`.
- `CLAMAV_SCAN_TIMEOUT_SECONDS`: default `120`.
- `MALWARE_SCANNER_FAILURE_POLICY`: `closed` or `open`; default `closed`.

Startup should fail fast when `MALWARE_SCANNER_PROVIDER=clamav` has invalid numeric timeout/port configuration. If application settings enable scanning but the process is deployed with the scanner provider disabled, scan jobs should move affected files to `scan_error` under fail-closed policy and log an explicit configuration error.

## Data Model

Add scan fields to `document_files` because scan state is file-specific, not document-specific. A document may have several files with different statuses.

Recommended columns:

- `scan_status varchar not null default 'not_required'`
- `scan_requested boolean not null default false`
- `scan_scanner varchar null`
- `scan_scanner_version varchar null`
- `scan_signature_version varchar null`
- `scan_threat_name varchar null`
- `scan_error text null`
- `scan_started_at timestamptz null`
- `scan_completed_at timestamptz null`
- `scan_requested_by bigint null references users(id)`

Valid statuses:

- `not_required`: scanning was disabled or the uploader opted out.
- `pending`: a scan was requested but not started.
- `scanning`: a worker is actively scanning.
- `clean`: scanner accepted the file.
- `infected`: scanner found malware; `scan_threat_name` should be set when available.
- `scan_error`: scan failed or timed out.

Add a database check constraint for the status values and consistency constraints where practical, such as requiring `scan_requested = true` for every status other than `not_required`. Existing document files should migrate to `not_required` so existing deployments continue to work without retroactive blocking.

Expose these fields in `DocumentFileView` so the document properties page can show status per file. Also expose a derived boolean named `content_available` that is true only for `clean` and `not_required` statuses. This boolean should be computed from `scan_status` in one shared backend function or SQL expression, not stored as an independently mutable column. Storing both a mutable boolean and a status would make every status transition update two authorities and create a drift risk.

The same derived boolean should be used by UI code. UI components should not need to know every terminal and non-terminal scan status just to decide whether a preview/download/process action is enabled.

Avoid exposing `scan_error` details to non-admin users if the message may contain operational details from the scanner; a generic status is sufficient in normal document views.

## Upload Flow

Autofile currently parses multipart uploads into a local temporary file during the HTTP request, calculates size and SHA-256, uploads the temp file to S3, inserts `document_files`, and enqueues thumbnail/page-processing jobs in the same transaction.

With scanning enabled, the request flow becomes:

```text
Upload request
  -> buffer multipart field to local temp file for this request only
  -> decide scan_requested from app settings and upload checkbox
  -> upload the file once to its normal S3 object key
  -> insert document_files with scan_status=pending or not_required
  -> enqueue ScanDocumentFile if pending
  -> enqueue normal processing jobs only if not_required
  -> remove local temp file
  -> return the created document/file metadata
```

The local temp file remains request-scoped. It is not the durable handoff between upload and scan. S3 is the durable handoff.

The S3 object should use the same `s3_prefix/filename` layout as today. No quarantine prefix or object promotion is required. The safety boundary is enforced by code paths that read S3 objects: download, thumbnail generation, page extraction, OCR, classification, and any manual processing endpoints must consult the scan state before opening the object.

If the API fails to enqueue the scan job after uploading to S3, the surrounding workflow should follow the existing cleanup pattern: roll back the DB transaction and delete the uploaded object. If a scan job later fails, the row stays present with `scan_error` so users can see that the upload exists but is unavailable.

If a user creates a document with an uploaded file that requires scanning, the document itself can be returned immediately. It may have no processed pages or thumbnail until the file is clean. The UI should make the file status visible rather than hiding the document.

## Scan Job Flow

Add a background job, preferably on the medium queue unless testing shows scanning should be isolated:

```text
MediumJob::ScanDocumentFile { document_file_id }
```

The worker should:

1. Load the document file and confirm it is still `pending` or retryable `scan_error`.
2. Atomically mark it `scanning` with `scan_started_at = now()`.
3. Stream the S3 object into the configured `MalwareScanner`.
4. Update scan fields based on the scanner result.
5. If clean, enqueue normal post-upload processing jobs for that file.
6. If infected or scan-error under fail-closed policy, leave the file unavailable and do not enqueue processing jobs.
7. If scanner failure occurs under fail-open policy, mark the file `not_required` or a clearly named fail-open clean-equivalent status and enqueue processing jobs.

The first implementation should prefer fail-closed. Fail-open must be explicit because it can allow unscanned files into processing and download paths.

Apalis already retries failed jobs. Scanner errors should be classified carefully: transient connection failures may return job errors so retry policy applies; a completed infected verdict should be a successful job that marks the file `infected`; repeated timeout/failure after retries should end as `scan_error` through the job handler's terminal-error path or explicit attempt tracking.

## Scanner Abstraction

Introduce a narrow application-level abstraction such as:

```text
MalwareScanner
  scan(stream, metadata) -> ScanOutcome

ScanOutcome
  Clean { scanner, scanner_version, signature_version }
  Infected { scanner, scanner_version, signature_version, threat_name }
  Error { scanner, reason }
```

The abstraction should answer only whether uploaded bytes may be trusted. It should not expose ClamAV-specific commands to document ingestion code.

The initial `ClamAvScanner` provider should connect to `clamd` over TCP and use `INSTREAM`, chunking bytes from S3 into the clamd stream. This keeps ClamAV away from S3 credentials and works equally with RustFS, AWS S3, and S3-compatible stores.

The implementation should also query scanner/version information when feasible. ClamAV's `VERSION` command can provide engine and signature information. If parsing the signature version is unreliable, store the raw version string in `scan_scanner_version` initially and leave `scan_signature_version` null.

ClamAV TCP is unauthenticated and unencrypted. Compose can rely on the private Compose network. Helm deployments should expose ClamAV as a ClusterIP-only service, and documentation should recommend NetworkPolicy limiting connections to Autofile API/worker pods.

## File Availability Model

Every code path that reads document file bytes from S3 must enforce scan availability. A helper should centralize the rule and return a simple result that can be used by handlers, jobs, and UI response shaping:

```text
document_file_content_availability(scan_status):
  not_required -> Available
  clean -> Available
  pending -> Unavailable("File is waiting for virus scanning.")
  scanning -> Unavailable("File is being scanned for viruses.")
  infected -> Unavailable("File is blocked because virus scanning found a threat.")
  scan_error -> Unavailable("File is unavailable because virus scanning failed.")
```

API responses should expose the boolean part of this rule as `content_available`. Handlers and jobs should use the richer helper so they can return a useful conflict message or log reason. If Diesel query filtering is needed, add a small SQL expression helper or constant list for available statuses: `('not_required', 'clean')`.

This keeps three useful properties:

- The database has one scan-state authority: `scan_status`.
- The API/UI get a simple boolean for routine decisions.
- Error messages stay consistent across download, preview, processing, and document pages.

## Processing And Download Gates

Apply the availability helper before any S3 object read or job enqueue that will lead to an object read:

Apply this helper to:

- `GET /documents/{document_id}/files/{id}/download`
- download-ticket issuance and ticket-backed downloads
- thumbnail generation jobs
- file-page extraction jobs
- OCR jobs
- classification jobs that read extracted text from files
- manual endpoints that enqueue page processing, thumbnails, or classification

The manual enqueue endpoints should return a conflict-style API error when a requested action is blocked by scanning rather than silently enqueueing a job that will fail later.

When a file is `infected`, Autofile should retain the object by default so administrators can audit the event and decide what to do. Normal users should not be able to download it through the application. Deleting the document or document file should delete the object using the existing cleanup path.

## API And UI Behavior

Upload endpoints should accept an optional multipart field such as `virus_scan` or `scan_for_viruses`. If scanning is disabled in settings, the API should ignore the field and store `not_required`. If scanning is enabled and the field is omitted, use `virus_scan_by_default`.

Document and document-file responses should include scan status fields. The UI should show scan controls only when public settings report `virus_scanning_enabled = true`.

File-oriented display pages need a consistent blocked-file treatment. Any page or component that would normally preview a file, list extracted pages, show OCR/text extraction results, show a thumbnail, or offer a download/process action should check `content_available` from the API response. When false, it should render an inline message in the area where the file content or derived content would normally appear rather than showing an empty or broken state.

Suggested unavailable messages:

- `pending`: `This file is stored but is waiting for virus scanning. It will be available after a clean scan.`
- `scanning`: `This file is being scanned for viruses. It will be available after a clean scan.`
- `infected`: `This file is blocked because virus scanning found a threat.` Include the threat name when available.
- `scan_error`: `This file is unavailable because virus scanning failed. An administrator can retry the scan or review scanner configuration.`

Affected UI surfaces include the document properties/file metadata page, file download actions, thumbnail or preview panels, file-page/text/OCR display pages, and any manual processing buttons. Actions that cannot work until the file is available should be disabled with explanatory helper text or replaced by the same blocked-file message.

The document properties/file metadata UI should also provide a rescan or retry-scan action for files whose scan status is `infected` or `scan_error`, and optionally for `clean` or `not_required` files if administrators need to rescan after scanner-definition updates. The action should be hidden when virus scanning is disabled globally, disabled while a file is already `pending` or `scanning`, and should submit to an authenticated API endpoint that resets scan metadata as appropriate and enqueues a scan job. The UI should clearly distinguish this action from normal document processing actions because rescan is allowed even when `content_available` is false.

Upload pages should render a checkbox:

- Label: `Virus scan this upload`
- Default checked state: public setting `virus_scan_by_default`
- Hidden entirely when `virus_scanning_enabled = false`

The document properties page should show per-file scan status. Suggested display states:

- `Not scanned`: scanning was not required for this upload.
- `Pending scan`: upload is stored and waiting for a scanner worker.
- `Scanning`: scanner worker is active.
- `Clean`: file is available for processing and download.
- `Infected`: file is blocked; include the threat name when available.
- `Scan error`: file is blocked because scanning failed.

If virus scanning is disabled globally, hide scan-specific UI elements. Existing API fields may still exist but should not clutter the UI.

## Deployment

Docker Compose should add a concrete `clamav` service so the feature can be tested locally without a separate scanner installation. Use the ClamAV container image, for example `clamav/clamav:stable`, and set the Compose service/container name to `clamav` so the API can reach `clamav:3310` on the private Compose network.

The API service should include scanner environment examples:

```yaml
MALWARE_SCANNER_PROVIDER: clamav
CLAMAV_HOST: clamav
CLAMAV_PORT: "3310"
MALWARE_SCANNER_FAILURE_POLICY: closed
```

The API should depend on the `clamav` service for local testing, ideally using a ClamAV healthcheck if the image supports one. Because database settings default scanning to disabled, including the service and environment does not force users to scan uploads until an administrator enables virus scanning in application settings. Documentation should call out that ClamAV may take time to download/load signatures on first startup.

The Helm chart should not maintain first-party ClamAV Deployment and Service templates. Instead, bundle the Wiremind ClamAV chart from ArtifactHub as a chart dependency, gated by `clamav.enabled`, following the existing dependency pattern used for Valkey and RustFS. At the time of design, ArtifactHub lists the Wiremind `clamav` chart as version `3.7.3`; implementation should pin an explicit chart version in `Chart.yaml` and update `Chart.lock` with `helm dependency update`.

Suggested `Chart.yaml` dependency shape:

```yaml
dependencies:
  - name: clamav
    version: 3.7.3
    repository: <wiremind chart repository URL>
    condition: clamav.enabled
```

The Autofile chart should expose only the values needed to enable/configure the dependency and to point the API at the resulting service. Prefer passing Wiremind chart values through the standard `clamav:` values subtree rather than mirroring every ClamAV setting under a separate Autofile-specific schema.

Suggested values shape:

```yaml
virusScanning:
  provider: clamav # clamav | external | disabled
  clamavHost: clamav
  clamavPort: 3310

clamav:
  enabled: true
  # Wiremind clamav chart values pass through here.
```

When `virusScanning.provider=clamav`, `clamav.enabled` should normally be true and Autofile should configure the API to reach the bundled Wiremind ClamAV service at `virusScanning.clamavHost:virusScanning.clamavPort`. The default host should be `clamav` and the default port should be `3310`.

When `virusScanning.provider=external`, operators should set `clamav.enabled=false` and configure `virusScanning.clamavHost` and `virusScanning.clamavPort` for their external ClamAV endpoint. When `virusScanning.provider=disabled`, the chart should not configure scanner environment variables and should not require the Wiremind ClamAV dependency to be enabled. Chart validation should reject inconsistent combinations such as `provider=clamav` with `clamav.enabled=false` unless an explicit host override is intentionally supported.

Documentation should state that ClamAV signature databases are large and may take time to initialize. Readiness behavior should avoid sending scan jobs to ClamAV before it is ready.

## Security Considerations

- ClamAV TCP must not be exposed outside the private deployment network.
- The scanner should not receive S3 credentials; Autofile streams bytes to it.
- Scanner failures should default to fail-closed.
- Infected files should not be downloaded or processed through normal application paths.
- Status changes should be auditable through timestamps and scanner metadata.
- UI controls are not authorization controls; the API must enforce settings and scan gates.

## Migration And Compatibility

Existing files should migrate to `not_required` to avoid making old documents unusable. This preserves current behavior until administrators enable scanning for new uploads.

The first implementation does not need to rescan existing objects. A future rescan workflow can use stored scanner/signature metadata to identify stale scans.

API clients that do not send the new multipart field should continue to work. When scanning is enabled, their uploads follow `virus_scan_by_default`.

## Testing

API tests should cover:

- Settings CRUD for `virus_scanning_enabled` and `virus_scan_by_default`.
- Upload defaults when scanning is disabled, enabled/default-on, and enabled/default-off.
- Explicit per-upload checkbox override.
- `pending`, `scanning`, `infected`, and `scan_error` files return `content_available=false` in document-file responses.
- Any file where `content_available=false` blocks all normal file operations except deletion and explicit rescan/retry-scan operations.
- Unsafe files block download and download-ticket flows.
- Unsafe files block page-image retrieval, extracted text display, OCR display, thumbnail retrieval/generation, page processing, OCR processing, classification, and other operations that read the file or its derived unsafe content.
- Unsafe files can still be deleted.
- Unsafe files can be submitted for an explicit rescan/retry operation when that operation exists.
- Clean scan result enqueues page processing and thumbnail generation.
- Infected scan result blocks download and processing and records threat metadata.
- Scanner failure follows fail-closed/fail-open policy.

Scanner integration tests should avoid requiring a real ClamAV daemon for normal CI. Unit tests can use a fake `MalwareScanner`. A separately documented manual or ignored test can exercise a real `clamd` container.

UI tests should cover:

- Upload checkbox hidden when virus scanning is disabled.
- Upload checkbox default state follows `virus_scan_by_default` when enabled.
- Settings page only shows scan-by-default when scanning is enabled.
- Document properties page renders scan statuses.
- File-oriented display pages render an informational unavailable-file message when `content_available=false` instead of trying to render missing thumbnails, page images, extracted text, OCR text, previews, or download actions.
- File-oriented display pages render normal content/actions when `content_available=true`.
- Document properties/file metadata UI shows a rescan or retry-scan action for eligible files when virus scanning is enabled, hides it when virus scanning is disabled, and disables it while a file is already pending or actively scanning.

Deployment validation should cover Docker Compose config and Helm rendering for bundled Wiremind ClamAV and external ClamAV modes.

## Future Work

- Re-scan documents after signature updates or scanner-provider changes.
- Admin actions for retrying failed scans and deleting infected files.
- ICAP provider for enterprise scanner integration.
- MetaDefender or ESET provider for commercial/on-prem scanners.
- AWS GuardDuty S3 scanning mode for AWS-native deployments.
- YARA rules as a separate organization-specific detection layer.
