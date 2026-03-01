import { useQuery, type UseQueryResult } from '@tanstack/react-query';
import { apiFetchList, type ListParams, type ResourceList, type ResourceInput, HttpError, apiFetch } from '../api';
import { type Document } from '../models/document';

export type DocumentTypeInput = ResourceInput<DocumentType>;

export function useDocuments(params: ListParams): UseQueryResult<ResourceList<Document>, HttpError> {
  return useQuery({
    queryKey: ['document', 'list', params],
    queryFn: () => apiFetchList<Document>('api/v1/documents', params),
  });
}

export function useDocument(id: string | number, options = {}): UseQueryResult<Document, HttpError> {
  return useQuery({
    queryKey: ['document', 'get', {id}],
    enabled: !!id,
    ...options,
    queryFn: () => apiFetch<Document>(`api/v1/documents/${id}`),
  });
}
