export const MAX_CABINETS = 200;

export interface Cabinet {
  id: number
  slug: string
  name: string
  displayName?: string
  description: string
  parent_id: number | null
}
