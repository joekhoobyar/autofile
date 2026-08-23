import { useQuery, type UseQueryResult } from "@tanstack/react-query";

import { apiFetch, apiFetchRaw, HttpError, parseApiError } from "../api";
import type { AppInfo } from "../models/appInfo";

export function useAppInfo(): UseQueryResult<AppInfo, HttpError> {
  return useQuery({
    queryKey: ["app-info"],
    queryFn: () => apiFetch<AppInfo>("api/v1/about"),
  });
}

export function useAppLicense(enabled: boolean): UseQueryResult<string, HttpError> {
  return useQuery({
    queryKey: ["app-license"],
    enabled,
    queryFn: async () => {
      const res = await apiFetchRaw("api/v1/about/license", {
        headers: { Accept: "text/plain" },
      });

      if (!res.ok) {
        throw await parseApiError(res);
      }

      return res.text();
    },
  });
}
