import {
  useMutation,
  useQuery,
  useQueryClient,
  type UseMutationResult,
  type UseQueryResult,
} from "@tanstack/react-query";

import {
  HttpError,
  apiFetch,
  apiFetchList,
  apiMutate,
  type ListParams,
  type ResourceList,
} from "../api";
import type { User, UserUpdateInput } from "../models/user";

export function useUsers(params: ListParams): UseQueryResult<ResourceList<User>, HttpError> {
  return useQuery({
    queryKey: ["user", "list", params],
    queryFn: () => apiFetchList<User>("api/v1/users", params),
  });
}

export function useUser(id: string | number, options = {}): UseQueryResult<User, HttpError> {
  return useQuery({
    queryKey: ["user", "get", { id }],
    enabled: !!id,
    ...options,
    queryFn: () => apiFetch<User>(`api/v1/users/${id}`),
  });
}

export function useSaveUser(): UseMutationResult<User, HttpError, UserUpdateInput> {
  const qc = useQueryClient();

  return useMutation<User, HttpError, UserUpdateInput>({
    mutationFn: (input) => {
      const body = {
        email: input.email,
        display_name: input.display_name,
      };
      return apiMutate<User, typeof body>(`api/v1/users/${input.id}`, {
        method: "PATCH",
        body,
      });
    },

    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["user"] });
    },
  });
}

export function useDeleteUser(): UseMutationResult<void, HttpError, number> {
  const qc = useQueryClient();

  return useMutation<void, HttpError, number>({
    mutationFn: (id) => {
      return apiMutate<void, void>(`api/v1/users/${id}`, {
        method: "DELETE",
      });
    },

    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["user"] });
    },
  });
}
