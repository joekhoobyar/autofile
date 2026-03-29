import { useQuery, type UseQueryResult } from '@tanstack/react-query';
import { apiFetchList, type ResourceList, HttpError } from '../api';
import { type DocumentIndexValue, type DocumentIndexValueListParams } from '../models/documentIndex';

export function useDocumentIndexValues(documentIndexId: string | number, params: DocumentIndexValueListParams): UseQueryResult<ResourceList<DocumentIndexValue>, HttpError> {
  return useQuery({
    queryKey: ['documentIndexValue', 'list', documentIndexId, params],
    enabled: !!documentIndexId,
    queryFn: () => apiFetchList<DocumentIndexValue>(`api/v1/document-indexes/${documentIndexId}/values`, params),
  });
}
