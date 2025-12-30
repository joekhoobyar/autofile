export interface MetadataType {
  id: number
  slug: string
  name: string
  data_type: 'string' | 'integer' | 'float' | 'boolean' | 'date';
  description?: string
}
