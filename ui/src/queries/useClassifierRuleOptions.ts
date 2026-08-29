import { useQuery, type UseQueryResult } from '@tanstack/react-query';

import { apiFetchList, HttpError } from '../api';
import type { Cabinet } from '../models/cabinet';
import type { DocumentType } from '../models/documentType';
import type { MetadataType } from '../models/metadataType';
import type { Tag } from '../models/tag';

export type ClassifierRuleOptions = {
  metadataTypes: MetadataType[];
  documentTypes: DocumentType[];
  tags: Tag[];
  cabinets: Cabinet[];
};

async function fetchAll<T>(path: string): Promise<T[]> {
  const items: T[] = [];
  let page = 1;

  while (true) {
    const result = await apiFetchList<T>(path, { page, per_page: 200, sf: 'name' });
    items.push(...result.items);
    if (items.length >= result.total || result.items.length === 0) break;
    page += 1;
  }

  return items;
}

export function useClassifierRuleOptions(): UseQueryResult<ClassifierRuleOptions, HttpError> {
  return useQuery({
    queryKey: ['classifierBlock', 'editorOptions'],
    queryFn: async () => {
      const [metadataTypes, documentTypes, tags, cabinets] = await Promise.all([
        fetchAll<MetadataType>('api/v1/metadata-types'),
        fetchAll<DocumentType>('api/v1/document-types'),
        fetchAll<Tag>('api/v1/tags'),
        fetchAll<Cabinet>('api/v1/cabinets'),
      ]);
      const cabinetById = new Map(cabinets.map((cabinet) => [cabinet.id, cabinet]));
      const cabinetDisplayName = (cabinet: Cabinet) => {
        const names: string[] = [];
        const visited = new Set<number>();
        let current: Cabinet | undefined = cabinet;
        while (current && !visited.has(current.id)) {
          visited.add(current.id);
          names.unshift(current.name);
          current = current.parent_id === null ? undefined : cabinetById.get(current.parent_id);
        }
        return names.join(' / ');
      };

      return {
        metadataTypes: metadataTypes.sort((a, b) => a.name.localeCompare(b.name)),
        documentTypes: documentTypes.sort((a, b) => a.name.localeCompare(b.name)),
        tags: tags.sort((a, b) => a.name.localeCompare(b.name)),
        cabinets: cabinets
          .map((cabinet) => ({ ...cabinet, displayName: cabinetDisplayName(cabinet) }))
          .sort((a, b) => (a.displayName ?? a.name).localeCompare(b.displayName ?? b.name)),
      };
    },
  });
}
