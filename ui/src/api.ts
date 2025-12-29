export interface ResourceList<T> {
  total: number;
  page: number;
  per_page: number;
  items: T[];
}

export async function apiFetch<T>(url: string): Promise<T> {
  const res = await fetch(`http://localhost:8000/${url}`);

  if (!res.ok) {
    throw new Error(`HTTP ${res.status}`);
  }

  return res.json() as Promise<T>;
}
