export interface MetadataType {
  id: number
  slug: string
  name: string
  data_type: 'string' | 'date' | 'lookup';
  description?: string
  options?: DataTypeOptions
}

export interface LookupOptions {
  choices: string[]
}

export type DataTypeOptions = undefined | LookupOptions;

