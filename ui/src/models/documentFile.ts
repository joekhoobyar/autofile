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
