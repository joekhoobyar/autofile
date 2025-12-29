import { useQuery, type UseQueryResult } from '@tanstack/react-query';
import { apiFetch, type ResourceList } from '../api';
import { type MetadataType } from '../models/metadataType';

export function useMetadataTypes(): UseQueryResult<ResourceList<MetadataType>, Error> {
  return useQuery({
    queryKey: ['metadataType', 'list'],
    queryFn: () => apiFetch<ResourceList<MetadataType>>('metadata-types'),
  });
}
