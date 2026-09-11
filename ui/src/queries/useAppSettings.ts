import {
  useMutation,
  useQuery,
  useQueryClient,
  type UseMutationResult,
  type UseQueryResult,
} from "@tanstack/react-query";

import { HttpError, apiFetch, apiMutate } from "../api";
import type { AppSettings, AppSettingsUpdateInput, PublicSettings } from "../models/appSettings";

export function usePublicSettings(): UseQueryResult<PublicSettings, HttpError> {
  return useQuery({
    queryKey: ["public-settings"],
    queryFn: () => apiFetch<PublicSettings>("api/v1/public/settings", { retryOn401: false }),
  });
}

export function useAppSettings(): UseQueryResult<AppSettings, HttpError> {
  return useQuery({
    queryKey: ["app-settings"],
    queryFn: () => apiFetch<AppSettings>("api/v1/settings"),
  });
}

export function useSaveAppSettings(): UseMutationResult<
  AppSettings,
  HttpError,
  AppSettingsUpdateInput
> {
  const qc = useQueryClient();

  return useMutation<AppSettings, HttpError, AppSettingsUpdateInput>({
    mutationFn: (input) => {
      return apiMutate<AppSettings, AppSettingsUpdateInput>("api/v1/settings", {
        method: "PATCH",
        body: input,
      });
    },

    onSuccess: (settings) => {
      qc.setQueryData(["app-settings"], settings);
    },
  });
}
