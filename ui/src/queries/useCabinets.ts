import { useMutation, useQuery, useQueryClient, type UseMutationResult, type UseQueryResult } from '@tanstack/react-query';
import { apiFetchList, type ListParams, type ResourceList, type ResourceInput, HttpError, apiMutate, apiFetch } from '../api';
import { MAX_CABINETS, type Cabinet } from '../models/cabinet';
import type { TreeNode } from 'primereact/treenode';

export type CabinetInput = ResourceInput<Cabinet>;

export function useCabinets(params: ListParams): UseQueryResult<ResourceList<Cabinet>, HttpError> {
  return useQuery({
    queryKey: ['cabinet', 'list', params],
    queryFn: async () => {
      const response = await apiFetchList<Cabinet>('api/v1/cabinets', params);
      const cabinetMap = new Map<number, Cabinet>();

      response.items.forEach((cabinet) => {
        cabinetMap.set(cabinet.id, cabinet);
      });

      const buildDisplayName = (cabinet: Cabinet): string => {
        const parts: string[] = [];
        const visited = new Set<number>();
        let current: Cabinet | undefined = cabinet;

        while (current) {
          if (visited.has(current.id)) break;
          visited.add(current.id);
          parts.unshift(current.name);
          current = current.parent_id ? cabinetMap.get(current.parent_id) : undefined;
        }

        return parts.join(' / ');
      };

      return {
        ...response,
        items: response.items.map((cabinet) => ({
          ...cabinet,
          displayName: buildDisplayName(cabinet),
        })),
      };
    },
  });
}

export function useCabinetTree(): UseQueryResult<TreeNode[], HttpError> {
  return useQuery({
    queryKey: ['cabinet', 'tree'],
    queryFn: async () => {
      const response = await apiFetchList<Cabinet>('api/v1/cabinets', {page:1, per_page: MAX_CABINETS});

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
      const cabinets = await apiFetch<Cabinet>(`api/v1/cabinets/${id}`);
      return cabinets;
    },
  });
}

export function useSaveCabinet(): UseMutationResult<Cabinet, HttpError, CabinetInput> {
  const qc = useQueryClient();

  return useMutation<Cabinet, HttpError, CabinetInput>({
    mutationFn: async (input) => {
      if (input.id) {
        return apiMutate<Cabinet, CabinetInput>(`api/v1/cabinets/${input.id}`, {
          method: "PATCH",
          body: input,
        });
      }
      return apiMutate<Cabinet, Omit<CabinetInput, "id">>(`api/v1/cabinets`, {
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

export function useDeleteCabinet(): UseMutationResult<void, HttpError, number> {
  const qc = useQueryClient();

  return useMutation<void, HttpError, number>({
    mutationFn: async (input) => {
      return apiMutate<void, void>(`api/v1/cabinets/${input}`, {
        method: "DELETE",
      });
    },

    onSuccess: () => {
      // invalidate by prefix (works with table params in the queryKey)
      qc.invalidateQueries({ queryKey: ["cabinet"] });
    },
  });
}
