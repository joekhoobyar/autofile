import { useMutation, useQuery, useQueryClient, type UseMutationResult, type UseQueryResult } from '@tanstack/react-query';
import { apiFetchList, type ListParams, type ResourceList, type ResourceInput, HttpError, apiMutate, apiFetch } from '../api';
import { type Tag } from '../models/tag';

export type TagInput = ResourceInput<Tag>;

export function useTags(params: ListParams): UseQueryResult<ResourceList<Tag>, HttpError> {
  return useQuery({
    queryKey: ['tag', 'list', params],
    queryFn: async () => {
      const tags = await apiFetchList<Tag>('api/v1/tags', params);
      return tags;
    },
  });
}

export function useTag(id: string | number, options = {}): UseQueryResult<Tag, HttpError> {
  return useQuery({
    queryKey: ['tag', 'get', {id}],
    enabled: !!id,
    ...options,
    queryFn: async () => {
      const tag = await apiFetch<Tag>(`api/v1/tags/${id}`);
      return tag;
    },
  });
}

export function useSaveTag(): UseMutationResult<Tag, HttpError, TagInput> {
  const qc = useQueryClient();

  return useMutation<Tag, HttpError, TagInput>({
    mutationFn: async (input) => {
      if (input.id) {
        return apiMutate<Tag, TagInput>(`api/v1/tags/${input.id}`, {
          method: "PATCH",
          body: input,
        });
      }
      return apiMutate<Tag, Omit<TagInput, "id">>(`api/v1/tags`, {
        method: "POST",
        body: input,
      });
    },

    onSuccess: () => {
      // invalidate by prefix (works with table params in the queryKey)
      qc.invalidateQueries({ queryKey: ["tag"] });
    },
  });
}

export function useDeleteTag(): UseMutationResult<void, HttpError, number> {
  const qc = useQueryClient();

  return useMutation<void, HttpError, number>({
    mutationFn: async (input) => {
      return apiMutate<void, void>(`api/v1/tags/${input}`, {
        method: "DELETE",
      });
    },

    onSuccess: () => {
      // invalidate by prefix (works with table params in the queryKey)
      qc.invalidateQueries({ queryKey: ["tag"] });
    },
  });
}
