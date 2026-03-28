import { useMutation, useQuery, useQueryClient, type UseMutationResult, type UseQueryResult } from '@tanstack/react-query';
import { apiFetchList, type ListParams, type ResourceList, type ResourceInput, HttpError, apiMutate, apiFetch } from '../api';
import { type DocumentIndexTemplate } from '../models/documentIndex';
import type { TreeNode } from 'primereact/treenode';

export type DocumentIndexTemplateInput = ResourceInput<DocumentIndexTemplate>;

const MAX_DOCUMENT_INDEX_TEMPLATES = 200;

export function useDocumentIndexTemplates(documentIndexId: string | number, params: ListParams): UseQueryResult<ResourceList<DocumentIndexTemplate>, HttpError> {
  return useQuery({
    queryKey: ['documentIndexTemplate', 'list', documentIndexId, params],
    enabled: !!documentIndexId,
    queryFn: () => apiFetchList<DocumentIndexTemplate>(`api/v1/document-indexes/${documentIndexId}/templates`, params),
  });
}

export function useDocumentIndexTemplateTree(documentIndexId: string | number): UseQueryResult<TreeNode[], HttpError> {
  return useQuery({
    queryKey: ['documentIndexTemplate', 'tree', documentIndexId],
    enabled: !!documentIndexId,
    queryFn: async () => {
      const response = await apiFetchList<DocumentIndexTemplate>(
        `api/v1/document-indexes/${documentIndexId}/templates`,
        { page: 1, per_page: MAX_DOCUMENT_INDEX_TEMPLATES },
      );

      const nodeMap = new Map<number, TreeNode>();

      response.items.forEach((template: DocumentIndexTemplate) => {
        nodeMap.set(template.id, {
          id: String(template.id),
          key: String(template.id),
          data: template,
          leaf: true,
          expanded: false,
          children: [],
        });
      });

      const rootNodes: TreeNode[] = [];

      nodeMap.forEach(node => {
        if (node.data.parent_id === null) {
          rootNodes.push(node);
        } else {
          const parent = nodeMap.get(node.data.parent_id);
          if (parent) {
            parent.children!.push(node);
            parent.leaf = false;
          } else {
            rootNodes.push(node);
          }
        }
      });

      return rootNodes;
    },
  });
}

export function useDocumentIndexTemplate(documentIndexId: string | number, id: string | number, options = {}): UseQueryResult<DocumentIndexTemplate, HttpError> {
  return useQuery({
    queryKey: ['documentIndexTemplate', 'get', documentIndexId, {id}],
    enabled: !!documentIndexId && !!id,
    ...options,
    queryFn: async () => {
      const template = await apiFetch<DocumentIndexTemplate>(`api/v1/document-indexes/${documentIndexId}/templates/${id}`);
      return template;
    },
  });
}

export function useSaveDocumentIndexTemplate(documentIndexId: string | number): UseMutationResult<DocumentIndexTemplate, HttpError, DocumentIndexTemplateInput> {
  const qc = useQueryClient();

  return useMutation<DocumentIndexTemplate, HttpError, DocumentIndexTemplateInput>({
    mutationFn: async (input) => {
      if (input.id) {
        return apiMutate<DocumentIndexTemplate, DocumentIndexTemplateInput>(
          `api/v1/document-indexes/${documentIndexId}/templates/${input.id}`,
          {
            method: "PATCH",
            body: input,
          }
        );
      }
      return apiMutate<DocumentIndexTemplate, Omit<DocumentIndexTemplateInput, "id">>(
        `api/v1/document-indexes/${documentIndexId}/templates`,
        {
          method: "POST",
          body: input,
        }
      );
    },

    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["documentIndexTemplate", documentIndexId] });
    },
  });
}

export function useDeleteDocumentIndexTemplate(documentIndexId: string | number): UseMutationResult<void, HttpError, number> {
  const qc = useQueryClient();

  return useMutation<void, HttpError, number>({
    mutationFn: async (input) => {
      return apiMutate<void, void>(`api/v1/document-indexes/${documentIndexId}/templates/${input}`, {
        method: "DELETE",
      });
    },

    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["documentIndexTemplate", documentIndexId] });
    },
  });
}
