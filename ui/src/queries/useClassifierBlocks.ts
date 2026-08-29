import { useMutation, useQuery, useQueryClient, type UseMutationResult, type UseQueryResult } from '@tanstack/react-query';

import { apiFetch, apiFetchList, HttpError, apiMutate, type ListParams, type ResourceInput, type ResourceList } from '../api';
import { type ClassifierBlock, type ClassifierRules } from '../models/classifierBlock';

export type ClassifierBlockInput = ResourceInput<ClassifierBlock>;
export type ClassifierBlockReorderInput = { id: number; order: number };
export type ClassifierRulesValidationIssue = { path: string; code: string; message: string };
export type ClassifierPatternValidation = { path: string; capture_count: number };
export type ClassifierRulesValidation = {
  valid: boolean;
  issues: ClassifierRulesValidationIssue[];
  patterns: ClassifierPatternValidation[];
};

const REORDER_FEEDBACK_MIN_MS = 250;

export function validateClassifierRules(rules: ClassifierRules): Promise<ClassifierRulesValidation> {
  return apiMutate<ClassifierRulesValidation, ClassifierRules>('api/v1/classifier-blocks/validate', {
    method: 'POST',
    body: rules,
  });
}

export function useValidateClassifierRules(): UseMutationResult<ClassifierRulesValidation, HttpError, ClassifierRules> {
  return useMutation<ClassifierRulesValidation, HttpError, ClassifierRules>({
    mutationFn: validateClassifierRules,
  });
}

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

export function useReorderClassifierBlock(): UseMutationResult<ClassifierBlock, HttpError, ClassifierBlockReorderInput> {
  const qc = useQueryClient();

  return useMutation<ClassifierBlock, HttpError, ClassifierBlockReorderInput>({
    mutationFn: async ({ id, order }) => {
      const [result] = await Promise.all([
        apiMutate<ClassifierBlock, { order: number }>(`api/v1/classifier-blocks/${id}/reorder`, {
          method: 'POST',
          body: { order },
        }),
        new Promise((resolve) => window.setTimeout(resolve, REORDER_FEEDBACK_MIN_MS)),
      ]);

      return result;
    },
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['classifierBlock'] });
    },
  });
}
