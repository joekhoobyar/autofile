import { useMutation, useQuery, useQueryClient, type UseMutationResult, type UseQueryResult } from '@tanstack/react-query';
import { apiFetchList, type ListParams, type ResourceList, type ResourceInput, HttpError, apiMutate, apiFetch } from '../api';
import { type DocumentType } from '../models/documentType';

export type DocumentTypeInput = ResourceInput<DocumentType>;
export type DocumentTypesMap = Record<string, DocumentType>;

export function useDocumentTypes(params: ListParams): UseQueryResult<ResourceList<DocumentType>, HttpError> {
  return useQuery({
    queryKey: ['documentType', 'list', params],
    queryFn: () => apiFetchList<DocumentType>('api/v1/document-types', params),
  });
}

export function useDocumentType(id: string | number, options = {}): UseQueryResult<DocumentType, HttpError> {
  return useQuery({
    queryKey: ['documentType', 'get', {id}],
    enabled: !!id,
    ...options,
    queryFn: () => apiFetch<DocumentType>(`api/v1/document-types/${id}`),
  });
}

export function useDocumentTypesMap(options = {}): UseQueryResult<DocumentTypesMap, HttpError> {
  return useQuery({
    queryKey: ['documentType', 'map', 'id'],
    ...options,
    queryFn: async () => {
      const per_page = 200;
      let page = 1;
      let total = 0;
      let items: DocumentType[] = [];

      while (true) {
        const res = await apiFetchList<DocumentType>('api/v1/document-types', { page, per_page });
        total = res.total ?? total;
        if (res.items?.length) {
          items = items.concat(res.items);
        }

        if (!res.items?.length) break;
        if (items.length >= res.total) break;
        if (res.page && res.per_page && res.page * res.per_page >= res.total) break;
        page += 1;
      }

      return items.reduce((acc, item) => {
        acc[item.id.toString()] = item;
        return acc;
      }, {} as DocumentTypesMap);
    },
  });
}

export function useSaveDocumentType(): UseMutationResult<DocumentType, HttpError, DocumentTypeInput> {
  const qc = useQueryClient();

  return useMutation<DocumentType, HttpError, DocumentTypeInput>({
    mutationFn: async (input) => {
      if (input.id) {
        return apiMutate<DocumentType, DocumentTypeInput>(`api/v1/document-types/${input.id}`, {
          method: "PATCH",
          body: input,
        });
      }
      return apiMutate<DocumentType, Omit<DocumentTypeInput, "id">>(`api/v1/document-types`, {
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

export function useDeleteDocumentType(): UseMutationResult<void, HttpError, number> {
  const qc = useQueryClient();

  return useMutation<void, HttpError, number>({
    mutationFn: async (input) => {
      return apiMutate<void, void>(`api/v1/document-types/${input}`, {
        method: "DELETE",
      });
    },

    onSuccess: () => {
      // invalidate by prefix (works with table params in the queryKey)
      qc.invalidateQueries({ queryKey: ["documentType"] });
    },
  });
}