import {
  useMutation,
  useQuery,
  useQueryClient,
  type UseMutationResult,
  type UseQueryResult,
} from "@tanstack/react-query";

import { HttpError, apiFetch, apiMutate, setAccessToken } from "../api";
import { sessionFromAccessToken } from "../auth";
import type { AccessTokenResponse, AuthSession } from "../models/auth";
import type { PasswordChangeInput, ProfileUpdateInput } from "../models/profile";
import type { User } from "../models/user";

export function useProfile(options = {}): UseQueryResult<User, HttpError> {
  return useQuery({
    queryKey: ["profile"],
    ...options,
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

export function useChangePassword(): UseMutationResult<AccessTokenResponse, HttpError, PasswordChangeInput> {
  const qc = useQueryClient();

  return useMutation<AccessTokenResponse, HttpError, PasswordChangeInput>({
    mutationFn: (body) =>
      apiMutate<AccessTokenResponse, PasswordChangeInput>("api/v1/profile/password", {
        method: "POST",
        body,
      }),

    onSuccess: (data) => {
      setAccessToken(data.access_token);
      const session = sessionFromAccessToken(data.access_token);
      if (session) {
        qc.setQueryData<AuthSession>(["auth", "bootstrap"], session);
      }
      qc.invalidateQueries({ queryKey: ["profile"] });
      qc.invalidateQueries({ queryKey: ["user"] });
    },
  });
}
