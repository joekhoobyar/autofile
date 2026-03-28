import { useMutation, useQuery, useQueryClient, type UseMutationResult, type UseQueryResult } from '@tanstack/react-query';
import { apiFetchList, type ListParams, type ResourceList, type ResourceInput, HttpError, apiMutate, apiFetch } from '../api';
import { type DocumentIndex } from '../models/documentIndex';

export type DocumentIndexInput = ResourceInput<DocumentIndex>;

export function useDocumentIndexes(params: ListParams): UseQueryResult<ResourceList<DocumentIndex>, HttpError> {
  return useQuery({
    queryKey: ['documentIndex', 'list', params],
    queryFn: () => apiFetchList<DocumentIndex>('api/v1/document-indexes', params),
  });
}

export function useDocumentIndex(id: string | number, options = {}): UseQueryResult<DocumentIndex, HttpError> {
  return useQuery({
    queryKey: ['documentIndex', 'get', {id}],
    enabled: !!id,
    ...options,
    queryFn: () => apiFetch<DocumentIndex>(`api/v1/document-indexes/${id}`),
  });
}

export function useSaveDocumentIndex(): UseMutationResult<DocumentIndex, HttpError, DocumentIndexInput> {
  const qc = useQueryClient();

  return useMutation<DocumentIndex, HttpError, DocumentIndexInput>({
    mutationFn: async (input) => {
      if (input.id) {
        return apiMutate<DocumentIndex, DocumentIndexInput>(`api/v1/document-indexes/${input.id}`, {
          method: "PATCH",
          body: input,
        });
      }
      return apiMutate<DocumentIndex, Omit<DocumentIndexInput, "id">>(`api/v1/document-indexes`, {
        method: "POST",
        body: input,
      });
    },

    onSuccess: () => {
      // invalidate by prefix (works with table params in the queryKey)
      qc.invalidateQueries({ queryKey: ["documentIndex"] });
    },
  });
}

export function useDeleteDocumentIndex(): UseMutationResult<void, HttpError, number> {
  const qc = useQueryClient();

  return useMutation<void, HttpError, number>({
    mutationFn: async (input) => {
      return apiMutate<void, void>(`api/v1/document-indexes/${input}`, {
        method: "DELETE",
      });
    },

    onSuccess: () => {
      // invalidate by prefix (works with table params in the queryKey)
      qc.invalidateQueries({ queryKey: ["documentIndex"] });
    },
  });
}
