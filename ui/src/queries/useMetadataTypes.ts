import { useQuery, type UseQueryResult } from '@tanstack/react-query';
import { apiFetchList, type ListParams, type ResourceList } from '../api';
import { type MetadataType } from '../models/metadataType';

export function useMetadataTypes(params: ListParams): UseQueryResult<ResourceList<MetadataType>, Error> {
  return useQuery({
    queryKey: ['metadataType', 'list', params],
    queryFn: () => apiFetchList<MetadataType>('metadata-types', params),
  });
}
