export interface Document {
  id: number
  title: string
  document_type_id: number
  metadata: Record<string, string>
}
