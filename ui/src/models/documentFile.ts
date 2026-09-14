export interface DocumentFile {
  id: number;
  document_id: number;
  filename: string;
  content_type?: string;
  size: number;
  pages: number;
  scan_status: string;
  scan_requested: boolean;
  scan_scanner?: string;
  scan_scanner_version?: string;
  scan_signature_version?: string;
  scan_threat_name?: string;
  scan_started_at?: string;
  scan_completed_at?: string;
  /** Derived from scan_status: true only for clean and not_required files. */
  content_available: boolean;
  created_at: string;
  created_by: number;
  updated_at: string;
  updated_by: number;
}

export interface DocumentFilePage {
  document_file_id: number;
  page_number: number;
  text_content?: string;
}

export interface DocumentFileOcrPage {
  document_file_id: number;
  page_number: number;
  ocr_content?: string;
}

export function documentFileScanStatusLabel(file: Pick<DocumentFile, 'scan_status' | 'scan_threat_name'>): string {
  switch (file.scan_status) {
    case 'not_required':
      return 'Not scanned';
    case 'pending':
      return 'Pending scan';
    case 'scanning':
      return 'Scanning';
    case 'clean':
      return 'Clean';
    case 'infected':
      return file.scan_threat_name ? `Infected: ${file.scan_threat_name}` : 'Infected';
    case 'scan_error':
      return 'Scan error';
    default:
      return file.scan_status;
  }
}

export type DocumentFileScanStatusSeverity =
  | 'success'
  | 'info'
  | 'warning'
  | 'danger'
  | 'secondary'
  | 'contrast'
  | undefined;

export function documentFileScanStatusSeverity(
  file: Pick<DocumentFile, 'scan_status'>,
): DocumentFileScanStatusSeverity {
  switch (file.scan_status) {
    case 'clean':
      return 'success';
    case 'infected':
    case 'scan_error':
      return 'danger';
    case 'scanning':
      return 'info';
    case 'pending':
      return 'warning';
    case 'not_required':
    default:
      return 'secondary';
  }
}

export function unavailableDocumentFileMessage(file: Pick<DocumentFile, 'scan_status' | 'scan_threat_name'>): string {
  switch (file.scan_status) {
    case 'pending':
      return 'This file is stored but is waiting for virus scanning. It will be available after a clean scan.';
    case 'scanning':
      return 'This file is being scanned for viruses. It will be available after a clean scan.';
    case 'infected':
      return file.scan_threat_name
        ? `This file is blocked because virus scanning found a threat: ${file.scan_threat_name}.`
        : 'This file is blocked because virus scanning found a threat.';
    case 'scan_error':
      return 'This file is unavailable because virus scanning failed. You can retry the scan or an administrator can review the scanner configuration.';
    default:
      return 'This file is unavailable until virus scanning is complete.';
  }
}

export function canSubmitDocumentFileRescan(file: Pick<DocumentFile, 'scan_status'>): boolean {
  return file.scan_status !== 'pending' && file.scan_status !== 'scanning';
}
