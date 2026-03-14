export const API_HOST = 'http://localhost:8000';

export type ResourceBase = {
  id: number;
  createdAt: Date;
  updatedAt: Date;
}
export type ResourceInput<T> = Omit<Partial<T>, "id" | "createdAt" | "updatedAt"> & { id?: number };

export type ApiError = {
  message: string;
  status: number;
  details?: unknown;
};

export class HttpError extends Error {
  public readonly status: number;
  public readonly details?: unknown;

  constructor(message: string, status: number, details?: unknown) {
    super(message);
    this.name = "HttpError";
    this.status = status;
    this.details = details;
  }
}

export async function parseApiError(res: Response): Promise<HttpError> {
  const contentType = res.headers.get("content-type") ?? "";
  let details: unknown = undefined;

  try {
    details = contentType.includes("application/json")
      ? await res.json()
      : await res.text();
  } catch {
    // ignore parsing errors
  }

  const msg =
    typeof details === "string"
      ? details
      : (details as { message?: string } | undefined)?.message ?? res.statusText;

  throw new HttpError(msg || "Request failed", res.status, details);
}

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

type FetchOptions = RequestInit & { retryOn401?: boolean };

let accessToken: string | null = null;

export function setAccessToken(token: string | null) {
  accessToken = token;
}

export async function apiFetch<T>(url: string, init: FetchOptions = {}): Promise<T> {
  
  // Perform a JSON based API request.
  const headers = new Headers(init.headers);
  if (! headers.has('Accept'))
    headers.set("Accept", "application/json");
  if (init.body !== undefined)
    headers.set("Content-Type", "application/json");
  const res = await apiFetchRaw(url, { ...init, headers });

  // Return JSON response if successful, otherwise throw.
  if (res.ok) {
    if (res.status === 204) return undefined as T;
    return res.json();
  }
  throw await parseApiError(res);
}

export async function apiFetchRaw(url: string, init: FetchOptions = {}): Promise<Response> {
  const retryOn401 = init.retryOn401 ?? true;

  // Perform the request with authorization headers.
  const headers = new Headers(init.headers);
  if (accessToken)
    headers.set("Authorization", `Bearer ${accessToken}`);
  const res = await fetch(`${API_HOST}/${url}`, {
    ...init,
    headers,
    credentials: "include"
  });

  // If access token expired, try refresh once, then retry original request
  if (res.status === 401 && retryOn401) {
    const refreshed = await tryRefresh();
    if (refreshed) {
      return apiFetchRaw(url, { ...init, retryOn401: false });
    }
  }

  return res;
}

async function tryRefresh(): Promise<boolean> {
  try {
    const res = await fetch(`${API_HOST}/api/v1/auth/refresh`, {
      method: "POST",
      credentials: "include",
      headers: { "Accept": "application/json" },
    });
    if (!res.ok) return false;

    const data = await res.json();
    // expects { access_token, token_type, expires_in }
    setAccessToken(data.access_token);
    return true;
  } catch {
    return false;
  }
}

type ApiMutateOptions<TBody> = Omit<FetchOptions, "body"> & {
  body?: TBody;
};

export async function apiMutate<TResponse, TBody = unknown>(
  url: string,
  options: ApiMutateOptions<TBody> = {}
): Promise<TResponse> {
  const { body, ...rest } = options;
  return apiFetch(url, {
    ...rest,
    body: body !== undefined ? JSON.stringify(body) : undefined,
  });
}
