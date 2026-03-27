export interface Document {
  id: number
  title: string
  document_type_id: number
  metadata: Record<string, string>
  created_at: string;
  created_by: number;
  updated_at: string;
  updated_by: number;
  cabinet_ids?: number[];
  tag_ids?: number[];
}

export interface NewDocumentMetadata {
  metadata_type_id: number;
  value: string;
}

export interface DocumentMetadata {
  document_id: number;
  metadata_type_id: number;
  value: string;
  created_at: string;
  created_by: number;
  updated_at: string;
  updated_by: number;
}

export interface NewCabinetDocument {
  document_id: number;
}

export interface CabinetDocument {
  cabinet_id: number;
  document_id: number;
  created_at: string;
  created_by: number;
  updated_at: string;
  updated_by: number;
}

export interface NewTagDocument {
  document_id: number;
}

export interface TagDocument {
  tag_id: number;
  document_id: number;
  updated_at: string;
  updated_by: number;
}
