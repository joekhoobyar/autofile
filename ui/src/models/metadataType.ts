export interface MetadataType {
  id: number
  slug: string
  name: string
  data_type: 'string' | 'integer' | 'date';
  description?: string
}
