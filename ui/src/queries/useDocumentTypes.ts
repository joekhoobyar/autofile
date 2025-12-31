import { useMutation, useQuery, useQueryClient, type UseMutationResult, type UseQueryResult } from '@tanstack/react-query';
import { apiFetchList, type ListParams, type ResourceList, type ResourceInput, HttpError, apiMutate, apiFetch } from '../api';
import { type DocumentType } from '../models/documentType';

export type DocumentTypeInput = ResourceInput<DocumentType>;

export function useDocumentTypes(params: ListParams): UseQueryResult<ResourceList<DocumentType>, HttpError> {
  return useQuery({
    queryKey: ['documentType', 'list', params],
    queryFn: () => apiFetchList<DocumentType>('document-types', params),
  });
}

export function useDocumentType(id: string | number, options = {}): UseQueryResult<DocumentType, HttpError> {
  return useQuery({
    queryKey: ['documentType', 'get', {id}],
    enabled: !!id,
    ...options,
    queryFn: () => apiFetch<DocumentType>(`document-types/${id}`),
  });
}

export function useSaveDocumentType(): UseMutationResult<DocumentType, HttpError, DocumentTypeInput> {
  const qc = useQueryClient();

  return useMutation<DocumentType, HttpError, DocumentTypeInput>({
    mutationFn: async (input) => {
      if (input.id) {
        return apiMutate<DocumentType, DocumentTypeInput>(`document-types/${input.id}`, {
          method: "PATCH",
          body: input,
        });
      }
      return apiMutate<DocumentType, Omit<DocumentTypeInput, "id">>(`document-types`, {
        method: "POST",
        body: input,
      });
    },

    onSuccess: () => {
      // invalidate by prefix (works with table params in the queryKey)
      qc.invalidateQueries({ queryKey: ["documentType"] });
    },
  });
}