export interface DocumentIndex {
  id: number
  slug: string
  name: string
  description?: string
  enabled: boolean
}

export interface DocumentIndexTemplate {
    id: number
    template: string
    is_leaf: boolean
    enabled: boolean
    document_index_id: number
    parent_id?: number
}
