export interface ListParams {
  page?: number;
  per_page?: number;
  q?: string;
  sf?: string;
  sd?: boolean;
}

export interface ResourceList<T> {
  total: number;
  page: number;
  per_page: number;
  items: T[];
}

export async function apiFetchList<T>(url: string, params: ListParams): Promise<ResourceList<T>> {
  const qp : string[] = [];

  if (params) {
    if (params.page)
      qp.push(`page=${encodeURIComponent(params.page)}`);
    if (params.per_page)
      qp.push(`per_page=${encodeURIComponent(params.per_page)}`);
    if (params.q)
      qp.push(`q=${encodeURIComponent(params.q)}`);
    if (params.sf)
      qp.push(`sf=${encodeURIComponent(params.sf)}`);
    if (params.sd !== undefined && params.sd !== null)
      qp.push(`sd=${params.sd ? 'true' : 'false'}`);
  }

  const q = qp.length ? '?'+qp.join('&') : '';
  return apiFetch<ResourceList<T>>(`${url}${q}`);
}

export async function apiFetch<T>(url: string): Promise<T> {
  const res = await fetch(`http://localhost:8000/${url}`);

  if (!res.ok) {
    throw new Error(`HTTP ${res.status}`);
  }

  return res.json() as Promise<T>;
}
