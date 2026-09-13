# virus-scanning

## ADDED Requirements

### Requirement: Virus Scanning Application Settings

Autofile SHALL provide application settings that control whether virus scanning is enabled and whether uploads request scanning by default.

#### Scenario: Hide scan-by-default when scanning is disabled

- **GIVEN** an administrator opens application settings
- **WHEN** `virus_scanning_enabled` is false
- **THEN** the UI does not show the `virus_scan_by_default` control

#### Scenario: Show scan-by-default when scanning is enabled

- **GIVEN** an administrator opens application settings
- **WHEN** `virus_scanning_enabled` is true
- **THEN** the UI shows a `virus_scan_by_default` control

#### Scenario: Public settings expose upload policy

- **GIVEN** a user opens an upload page
- **WHEN** the UI loads public settings
- **THEN** the response includes `virus_scanning_enabled` and `virus_scan_by_default`

### Requirement: Per-Upload Scan Selection

Autofile SHALL let users choose whether an upload requests virus scanning when virus scanning is enabled.

#### Scenario: Upload checkbox hidden when scanning is disabled

- **GIVEN** public settings have `virus_scanning_enabled` set to false
- **WHEN** a user opens a document upload form
- **THEN** the form does not show a virus-scan checkbox

#### Scenario: Upload checkbox follows default setting

- **GIVEN** public settings have `virus_scanning_enabled` set to true
- **AND** `virus_scan_by_default` is true
- **WHEN** a user opens a document upload form
- **THEN** the form shows `Virus scan this upload` checked by default

#### Scenario: User opts out of scan for one upload

- **GIVEN** virus scanning is enabled
- **AND** the user unchecks `Virus scan this upload`
- **WHEN** the user submits the upload
- **THEN** Autofile stores the uploaded file with scan status `not_required`
- **AND** Autofile may enqueue normal document-processing jobs immediately

### Requirement: Durable Pending Scan Uploads

Autofile SHALL store uploaded files durably in object storage before asynchronous virus scanning proceeds.

#### Scenario: Scanned upload survives API restart

- **GIVEN** virus scanning is enabled
- **AND** an upload requests scanning
- **WHEN** the upload API call succeeds
- **THEN** the uploaded file bytes are stored in S3-compatible object storage
- **AND** the document file has scan status `pending`
- **AND** scanning can proceed later without relying on local temporary upload files

#### Scenario: No object promotion copy is required

- **GIVEN** an upload requests scanning
- **WHEN** Autofile stores the uploaded object
- **THEN** Autofile uses the normal document-file object key
- **AND** Autofile controls file availability through document-file scan status rather than copying the object from a quarantine prefix after scan

### Requirement: Asynchronous Scan Job

Autofile SHALL process requested virus scans through a background job that streams the stored object to the configured scanner.

#### Scenario: Clean scan releases file for processing

- **GIVEN** a document file has scan status `pending`
- **WHEN** the scan job receives a clean scanner verdict
- **THEN** Autofile updates the file scan status to `clean`
- **AND** records scanner metadata and completion time when available
- **AND** enqueues normal post-upload processing jobs for the file

#### Scenario: Infected scan blocks file use

- **GIVEN** a document file has scan status `pending`
- **WHEN** the scan job receives an infected scanner verdict
- **THEN** Autofile updates the file scan status to `infected`
- **AND** records the threat name when available
- **AND** does not enqueue document-processing jobs for the file

#### Scenario: Scan error fails closed by default

- **GIVEN** the scanner is unavailable
- **AND** the failure policy is `closed`
- **WHEN** a scan job cannot complete successfully
- **THEN** Autofile updates the file scan status to `scan_error`
- **AND** does not enqueue document-processing jobs for the file

### Requirement: Content Access Gates

Autofile SHALL prevent document-file content from being downloaded or processed until required virus scanning succeeds.

#### Scenario: Availability boolean is derived from scan status

- **GIVEN** a document file has scan status `clean` or `not_required`
- **WHEN** Autofile returns document-file metadata
- **THEN** the response includes a content-availability boolean set to true
- **AND** the boolean is derived from scan status rather than maintained as an independent status authority

#### Scenario: Unavailable status derives false availability

- **GIVEN** a document file has scan status `pending`, `scanning`, `infected`, or `scan_error`
- **WHEN** Autofile returns document-file metadata
- **THEN** the response includes a content-availability boolean set to false
- **AND** handlers and jobs reject operations that would read file bytes

#### Scenario: Unsafe file operations are rejected

- **GIVEN** a document file has content availability set to false
- **WHEN** a user or worker requests download, download ticket creation, page image retrieval, extracted text retrieval, OCR text retrieval, thumbnail retrieval, thumbnail generation, page processing, OCR processing, classification, or another operation that reads file bytes or unsafe derived content
- **THEN** Autofile rejects or skips the operation without reading file bytes
- **AND** Autofile returns or records a message explaining that the file is unavailable until there is a clean scan

#### Scenario: Unsafe file can be deleted or rescanned

- **GIVEN** a document file has content availability set to false
- **WHEN** an authorized user deletes the file or requests an explicit rescan/retry-scan operation
- **THEN** Autofile allows the operation subject to existing authorization and validation rules

#### Scenario: Pending file cannot be downloaded

- **GIVEN** a document file has scan status `pending`
- **WHEN** a user requests a download for that file
- **THEN** Autofile rejects the request without serving object bytes

#### Scenario: Infected file cannot be processed

- **GIVEN** a document file has scan status `infected`
- **WHEN** a user or worker requests thumbnail generation, page extraction, OCR, or classification that would read the file
- **THEN** Autofile rejects or skips the operation without reading the object bytes

#### Scenario: Clean file can be used normally

- **GIVEN** a document file has scan status `clean`
- **WHEN** a user downloads the file or Autofile processes it
- **THEN** Autofile allows the operation subject to existing authorization and validation rules

#### Scenario: Not-required file can be used normally

- **GIVEN** a document file has scan status `not_required`
- **WHEN** a user downloads the file or Autofile processes it
- **THEN** Autofile allows the operation subject to existing authorization and validation rules

### Requirement: Scan Status Display

Autofile SHALL display per-file virus-scan status and unavailable-file messages on file-oriented pages when virus scanning is enabled.

#### Scenario: Show pending scan status

- **GIVEN** virus scanning is enabled
- **AND** a document file has scan status `pending`
- **WHEN** a user views the document properties page
- **THEN** the UI displays that the file is pending virus scan

#### Scenario: Show unavailable message instead of file content

- **GIVEN** virus scanning is enabled
- **AND** a document file has scan status `pending`, `scanning`, `infected`, or `scan_error`
- **WHEN** a user opens a page or panel that would normally preview, download, display extracted pages, display text/OCR, or manually process the file
- **THEN** the UI does not show an empty or broken file-content state
- **AND** the UI displays a message explaining that the file is unavailable until there is a clean scan

#### Scenario: UI tests cover unavailable file content

- **GIVEN** a file-oriented UI component receives document-file data with content availability set to false
- **WHEN** the component renders
- **THEN** automated UI tests verify that it displays an informational unavailable-file message
- **AND** automated UI tests verify that it does not attempt to render normal file content or enabled file-content actions

#### Scenario: Submit file for rescan from the UI

- **GIVEN** virus scanning is enabled
- **AND** a document file is eligible for rescan or retry-scan
- **WHEN** a user views the document properties or file metadata UI
- **THEN** the UI provides an action to submit that file for rescan
- **AND** the action remains available even when `content_available=false`

#### Scenario: Hide or disable rescan action when unavailable

- **GIVEN** virus scanning is disabled, or a document file is already pending or actively scanning
- **WHEN** a user views the document properties or file metadata UI
- **THEN** the UI hides or disables the rescan action with appropriate explanatory text

#### Scenario: Hide scan UI when scanning is disabled

- **GIVEN** virus scanning is disabled
- **WHEN** a user views the document properties page
- **THEN** the UI does not show virus-scan-specific status elements

#### Scenario: Show infected status with threat name

- **GIVEN** virus scanning is enabled
- **AND** a document file has scan status `infected`
- **AND** a threat name is available
- **WHEN** a user views the document properties page
- **THEN** the UI displays that the file is infected
- **AND** includes the threat name

### Requirement: ClamAV Provider

Autofile SHALL support ClamAV `clamd` as the initial malware scanner provider.

#### Scenario: Stream object bytes to clamd

- **GIVEN** the scanner provider is configured as `clamav`
- **AND** a scan job starts for a document file
- **WHEN** Autofile scans the file
- **THEN** Autofile streams the S3 object bytes to `clamd` using the `INSTREAM` protocol
- **AND** ClamAV does not require direct S3 credentials

#### Scenario: Invalid ClamAV configuration fails fast

- **GIVEN** the scanner provider is configured as `clamav`
- **AND** ClamAV port or timeout configuration is invalid
- **WHEN** the API starts
- **THEN** startup fails with a clear configuration error

#### Scenario: Docker Compose includes local ClamAV

- **GIVEN** a developer starts the Docker Compose stack
- **WHEN** virus scanning is enabled in application settings
- **THEN** Autofile can reach a Compose `clamav` service on the private Compose network
- **AND** scan jobs can test against that local ClamAV container

#### Scenario: Helm bundles Wiremind ClamAV chart when enabled

- **GIVEN** the Helm value `virusScanning.provider` is `clamav`
- **AND** the Helm value `clamav.enabled` is true
- **WHEN** the Autofile chart dependencies are built and the chart is rendered
- **THEN** Autofile includes the Wiremind ClamAV chart as a dependency
- **AND** the API is configured to reach `virusScanning.clamavHost:virusScanning.clamavPort`

#### Scenario: Helm uses external ClamAV when bundled chart is disabled

- **GIVEN** the Helm value `virusScanning.provider` is `external`
- **AND** the Helm value `clamav.enabled` is false
- **AND** `virusScanning.clamavHost` and `virusScanning.clamavPort` are configured
- **WHEN** the Autofile chart is rendered
- **THEN** Autofile does not render first-party ClamAV Deployment or Service templates
- **AND** the API is configured to reach the external ClamAV endpoint

#### Scenario: Helm disables scanner configuration

- **GIVEN** the Helm value `virusScanning.provider` is `disabled`
- **WHEN** the Autofile chart is rendered
- **THEN** Autofile does not require the Wiremind ClamAV chart to be enabled
- **AND** the API is not configured to contact a malware scanner
