export interface DocumentFile {
  id: number;
  document_id: number;
  filename: string;
  content_type?: string;
  size: number;
  pages: number;
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
