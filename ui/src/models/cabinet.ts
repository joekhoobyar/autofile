export const MAX_CABINETS = 1000;

export interface Cabinet {
  id: number
  slug: string
  name: string
  displayName?: string
  description: string
  parent_id: number | null
  document_count: number
}
