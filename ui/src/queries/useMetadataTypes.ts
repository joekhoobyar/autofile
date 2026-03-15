import { useMutation, useQuery, useQueryClient, type UseMutationResult, type UseQueryResult } from '@tanstack/react-query';
import { apiFetchList, type ListParams, type ResourceList, type ResourceInput, HttpError, apiMutate, apiFetch } from '../api';
import { type MetadataType } from '../models/metadataType';
import { type DocumentTypeMetadataType } from '../models/documentTypeMetadataType';

export type MetadataTypeInput = ResourceInput<MetadataType>;
export type MetadataTypesMap = Record<string, MetadataType>;

export function useDocumentTypeMetadataTypes(document_type_id: string | number | undefined, options = {}): UseQueryResult<DocumentTypeMetadataType[], HttpError> {
  return useQuery({
    queryKey: ['documentTypeMetadataType', 'list', {document_type_id}],
    enabled: !!document_type_id,
    ...options,
    queryFn: () => apiFetch<DocumentTypeMetadataType[]>(`api/v1/document-types-metadata-types?document_type_id=${document_type_id}`),
  });
}

export function useMetadataTypes(params: ListParams): UseQueryResult<ResourceList<MetadataType>, HttpError> {
  return useQuery({
    queryKey: ['metadataType', 'list', params],
    queryFn: () => apiFetchList<MetadataType>('api/v1/metadata-types', params),
  });
}

export function useMetadataType(id: string | number, options = {}): UseQueryResult<MetadataType, HttpError> {
  return useQuery({
    queryKey: ['metadataType', 'get', {id}],
    enabled: !!id,
    ...options,
    queryFn: () => apiFetch<MetadataType>(`api/v1/metadata-types/${id}`),
  });
}

export function useMetadataTypesMap(by: 'slug' | 'id', options = {}): UseQueryResult<MetadataTypesMap, HttpError> {
  return useQuery({
    queryKey: ['metadataType', 'map', by],
    ...options,
    queryFn: async () => {
      const per_page = 200;
      let page = 1;
      let total = 0;
      let items: MetadataType[] = [];

      while (true) {
        const res = await apiFetchList<MetadataType>('api/v1/metadata-types', { page, per_page });
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
        acc[item[by].toString()] = item;
        return acc;
      }, {} as MetadataTypesMap);
    },
  });
}

export function useSaveMetadataType(): UseMutationResult<MetadataType, HttpError, MetadataTypeInput> {
  const qc = useQueryClient();

  return useMutation<MetadataType, HttpError, MetadataTypeInput>({
    mutationFn: async (input) => {
      if (input.id) {
        return apiMutate<MetadataType, MetadataTypeInput>(`api/v1/metadata-types/${input.id}`, {
          method: "PATCH",
          body: input,
        });
      }
      return apiMutate<MetadataType, Omit<MetadataTypeInput, "id">>(`api/v1/metadata-types`, {
        method: "POST",
        body: input,
      });
    },

    onSuccess: () => {
      // invalidate by prefix (works with table params in the queryKey)
      qc.invalidateQueries({ queryKey: ["metadataType"] });
    },
  });
}

export function useDeleteMetadataType(): UseMutationResult<void, HttpError, number> {
  const qc = useQueryClient();

  return useMutation<void, HttpError, number>({
    mutationFn: async (input) => {
      return apiMutate<void, void>(`api/v1/metadata-types/${input}`, {
        method: "DELETE",
      });
    },

    onSuccess: () => {
      // invalidate by prefix (works with table params in the queryKey)
      qc.invalidateQueries({ queryKey: ["metadataType"] });
    },
  });
}
