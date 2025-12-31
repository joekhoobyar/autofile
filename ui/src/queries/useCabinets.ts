import { useMutation, useQuery, useQueryClient, type UseMutationResult, type UseQueryResult } from '@tanstack/react-query';
import { apiFetchList, type ListParams, type ResourceList, type ResourceInput, HttpError, apiMutate, apiFetch } from '../api';
import { MAX_CABINETS, type Cabinet } from '../models/cabinet';
import type { TreeNode } from 'primereact/treenode';

export type CabinetInput = ResourceInput<Cabinet>;

export function useCabinets(params: ListParams): UseQueryResult<ResourceList<Cabinet>, HttpError> {
  return useQuery({
    queryKey: ['cabinet', 'list', params],
    queryFn: () => apiFetchList<Cabinet>('cabinets', params),
  });
}

export function useCabinetTree(): UseQueryResult<TreeNode[], HttpError> {
  return useQuery({
    queryKey: ['cabinet', 'tree'],
    queryFn: async () => {
      const response = await apiFetchList<Cabinet>('cabinets', {page:1, per_page: MAX_CABINETS});

      // Convert Cabinet to CabinetNode
      const nodeMap = new Map<number, TreeNode>();

      response.items.forEach((cabinet: Cabinet) => {
        nodeMap.set(cabinet.id, {
          id: String(cabinet.id),
          key: cabinet.slug,
          data: cabinet,
          leaf: true,
          expanded: false,
          children: [],
        });
      });

      // Build tree structure
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
            // Parent not found, treat as root
            rootNodes.push(node);
          }
        }
      });

      return rootNodes;
    },
  });
}

export function useCabinet(id: string | number, options = {}): UseQueryResult<Cabinet, HttpError> {
  return useQuery({
    queryKey: ['cabinet', 'get', {id}],
    enabled: !!id,
    ...options,
    queryFn: async () => {
      const cabinets = await apiFetch<Cabinet>(`cabinets/${id}`);
      return cabinets;
    },
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