import { useMutation, useQuery, useQueryClient, type UseMutationResult, type UseQueryResult } from '@tanstack/react-query';
import { apiFetchList, type ListParams, type ResourceList, type ResourceInput, HttpError, apiMutate, apiFetch } from '../api';
import { type MetadataType } from '../models/metadataType';

export type MetadataTypeInput = ResourceInput<MetadataType>;

export function useMetadataTypes(params: ListParams): UseQueryResult<ResourceList<MetadataType>, HttpError> {
  return useQuery({
    queryKey: ['metadataType', 'list', params],
    queryFn: () => apiFetchList<MetadataType>('metadata-types', params),
  });
}

export function useMetadataType(id: string | number): UseQueryResult<MetadataType, HttpError> {
  return useQuery({
    queryKey: ['metadataType', 'get', {id}],
    enabled: !!id,
    queryFn: () => apiFetch<MetadataType>(`metadata-types/${id}`),
  });
}

export function useSaveMetadataType(): UseMutationResult<MetadataType, HttpError, MetadataTypeInput> {
  const qc = useQueryClient();

  return useMutation<MetadataType, HttpError, MetadataTypeInput>({
    mutationFn: async (input) => {
      if (input.id) {
        return apiMutate<MetadataType, MetadataTypeInput>(`metadata-types/${input.id}`, {
          method: "PATCH",
          body: input,
        });
      }
      return apiMutate<MetadataType, Omit<MetadataTypeInput, "id">>(`metadata-types`, {
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