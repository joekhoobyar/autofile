export interface DocumentIndex {
  id: number
  slug: string
  name: string
  description?: string
  enabled: boolean
}

export interface DocumentIndexView {
  id: number
  slug: string
  name: string
  description?: string
  enabled: boolean
  document_count: number
}

export interface DocumentIndexTemplate {
    id: number
    template: string
    is_leaf: boolean
    enabled: boolean
    document_index_id: number
    parent_id?: number
}

export interface DocumentIndexValue {
    id: number
    value: string
    is_leaf: boolean
    document_index_id: number
    document_index_template_id: number
    parent_id?: number
}

export interface DocumentIndexValueListParams {
  page?: number;
  per_page?: number;
  q?: string;
  parent_id?: number | null | 'null';

  // sf = sort field
  sf?: string;

  // sd = sort descending
  sd?: boolean;
}
