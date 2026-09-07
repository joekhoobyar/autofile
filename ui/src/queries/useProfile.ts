import {
  useMutation,
  useQuery,
  useQueryClient,
  type UseMutationResult,
  type UseQueryResult,
} from "@tanstack/react-query";

import { HttpError, apiFetch, apiMutate } from "../api";
import type { PasswordChangeInput, ProfileUpdateInput } from "../models/profile";
import type { User } from "../models/user";

export function useProfile(): UseQueryResult<User, HttpError> {
  return useQuery({
    queryKey: ["profile"],
    queryFn: () => apiFetch<User>("api/v1/profile"),
  });
}

export function useSaveProfile(): UseMutationResult<User, HttpError, ProfileUpdateInput> {
  const qc = useQueryClient();

  return useMutation<User, HttpError, ProfileUpdateInput>({
    mutationFn: (body) =>
      apiMutate<User, ProfileUpdateInput>("api/v1/profile", {
        method: "PATCH",
        body,
      }),

    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["profile"] });
      qc.invalidateQueries({ queryKey: ["user"] });
    },
  });
}

export function useChangePassword(): UseMutationResult<User, HttpError, PasswordChangeInput> {
  const qc = useQueryClient();

  return useMutation<User, HttpError, PasswordChangeInput>({
    mutationFn: (body) =>
      apiMutate<User, PasswordChangeInput>("api/v1/profile/password", {
        method: "POST",
        body,
      }),

    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["profile"] });
      qc.invalidateQueries({ queryKey: ["user"] });
    },
  });
}
