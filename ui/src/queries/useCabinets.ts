import { useMutation, useQuery, useQueryClient, type UseMutationResult, type UseQueryResult } from '@tanstack/react-query';
import { apiFetchList, type ListParams, type ResourceList, type ResourceInput, HttpError, apiMutate, apiFetch } from '../api';
import { type Cabinet } from '../models/cabinet';

export type CabinetInput = ResourceInput<Cabinet>;

export function useCabinets(params: ListParams): UseQueryResult<ResourceList<Cabinet>, HttpError> {
  return useQuery({
    queryKey: ['cabinet', 'list', params],
    queryFn: () => apiFetchList<Cabinet>('cabinets', params),
  });
}

export function useCabinet(id: string | number): UseQueryResult<Cabinet, HttpError> {
  return useQuery({
    queryKey: ['cabinet', 'get', {id}],
    enabled: !!id,
    queryFn: () => apiFetch<Cabinet>(`cabinets/${id}`),
  });
}

export function useSaveCabinet(): UseMutationResult<Cabinet, HttpError, CabinetInput> {
  const qc = useQueryClient();

  return useMutation<Cabinet, HttpError, CabinetInput>({
    mutationFn: async (input) => {
      if (input.id) {
        return apiMutate<Cabinet, CabinetInput>(`cabinets/${input.id}`, {
          method: "PATCH",
          body: input,
        });
      }
      return apiMutate<Cabinet, Omit<CabinetInput, "id">>(`cabinets`, {
        method: "POST",
        body: input,
      });
    },

    onSuccess: () => {
      // invalidate by prefix (works with table params in the queryKey)
      qc.invalidateQueries({ queryKey: ["cabinet"] });
    },
  });
}