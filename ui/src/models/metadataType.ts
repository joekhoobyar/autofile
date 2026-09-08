export interface MetadataType {
  id: number
  slug: string
  name: string
  data_type: 'string' | 'date' | 'lookup';
  description?: string
  options?: DataTypeOptions
  created_at: string
  updated_at: string
}

export interface LookupOptions {
  choices: string[]
}

export type DataTypeOptions = undefined | LookupOptions;
