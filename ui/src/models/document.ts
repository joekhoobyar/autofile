export interface Document {
  id: number
  title: string
  document_type_id: number
  metadata: Record<string, string>
  created_at: string;
  created_by: number;
  updated_at: string;
  updated_by: number;
}
