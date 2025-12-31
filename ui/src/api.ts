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

export async function apiFetch<T>(url: string): Promise<T> {
  const res = await fetch(`http://localhost:8000/${url}`, {
    headers: {
      Accept: "application/json",
    }
  });

  if (!res.ok) {
    throw await parseApiError(res);
  }

  return res.json();
}


type ApiMutateOptions<TBody> = Omit<RequestInit, "body"> & {
  body?: TBody;
};

export async function apiMutate<TResponse, TBody = unknown>(
  url: string,
  options: ApiMutateOptions<TBody> = {}
): Promise<TResponse> {
  const { body, headers, ...rest } = options;

  const res = await fetch(`http://localhost:8000/${url}`, {
    ...rest,
    headers: {
      Accept: "application/json",
      ...(body !== undefined ? { "Content-Type": "application/json" } : {}),
      ...(headers ?? {}),
    },
    ...options,
    body: body !== undefined ? JSON.stringify(body) : undefined,
  });

  if (!res.ok) {
    throw await parseApiError(res);
  }

  // Some endpoints return 204 No Content
  if (res.status === 204) {
    return undefined as TResponse;
  }

  return res.json();
}
