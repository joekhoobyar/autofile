import { useMutation, useQuery, useQueryClient, type UseMutationResult, type UseQueryResult } from '@tanstack/react-query';

import { apiFetch, apiFetchList, HttpError, apiMutate, type ListParams, type ResourceInput, type ResourceList } from '../api';
import { type ClassifierBlock } from '../models/classifierBlock';

export type ClassifierBlockInput = ResourceInput<ClassifierBlock>;

export function useClassifierBlocks(params: ListParams): UseQueryResult<ResourceList<ClassifierBlock>, HttpError> {
  return useQuery({
    queryKey: ['classifierBlock', 'list', params],
    queryFn: () => apiFetchList<ClassifierBlock>('api/v1/classifier-blocks', params),
  });
}

export function useClassifierBlock(id: string | number, options = {}): UseQueryResult<ClassifierBlock, HttpError> {
  return useQuery({
    queryKey: ['classifierBlock', 'get', { id }],
    enabled: !!id,
    ...options,
    queryFn: () => apiFetch<ClassifierBlock>(`api/v1/classifier-blocks/${id}`),
  });
}

export function useSaveClassifierBlock(): UseMutationResult<ClassifierBlock, HttpError, ClassifierBlockInput> {
  const qc = useQueryClient();

  return useMutation<ClassifierBlock, HttpError, ClassifierBlockInput>({
    mutationFn: async (input) => {
      if (input.id) {
        return apiMutate<ClassifierBlock, ClassifierBlockInput>(`api/v1/classifier-blocks/${input.id}`, {
          method: 'PATCH',
          body: input,
        });
      }

      return apiMutate<ClassifierBlock, Omit<ClassifierBlockInput, 'id'>>('api/v1/classifier-blocks', {
        method: 'POST',
        body: input,
      });
    },
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['classifierBlock'] });
    },
  });
}

export function useDeleteClassifierBlock(): UseMutationResult<void, HttpError, number> {
  const qc = useQueryClient();

  return useMutation<void, HttpError, number>({
    mutationFn: async (input) => {
      return apiMutate<void, void>(`api/v1/classifier-blocks/${input}`, {
        method: 'DELETE',
      });
    },
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['classifierBlock'] });
    },
  });
}
