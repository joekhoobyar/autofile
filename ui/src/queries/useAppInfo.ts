import { useQuery, type UseQueryResult } from "@tanstack/react-query";

import { apiFetch, HttpError } from "../api";
import type { AppInfo } from "../models/appInfo";

export function useAppInfo(): UseQueryResult<AppInfo, HttpError> {
  return useQuery({
    queryKey: ["app-info"],
    queryFn: () => apiFetch<AppInfo>("api/v1/about"),
  });
}
