import { useQuery, type UseQueryResult } from '@tanstack/react-query';
import { apiFetchList, type ListParams, type ResourceList, type ResourceInput, HttpError, apiFetch, apiFetchRaw } from '../api';
import { type Document } from '../models/document';
import { useEffect, useMemo } from 'react';

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

export function useDocumentThumbnail(id: string | number, options = {}): UseQueryResult<string | undefined, HttpError> {
  const query = useQuery<Blob, HttpError>({
    queryKey: ['document', 'get', {id}, 'thumbnail'],
    enabled: !!id,
    ...options,
    queryFn: async () => {
      const res = await apiFetchRaw(`api/v1/documents/${id}/thumbnail`);
      return await res.blob();
    }
  });

  const objectUrl = useMemo(() => {
    if (!query.data) return undefined;
    return URL.createObjectURL(query.data);
  }, [query.data]);

  useEffect(() => {
    return () => {
      if (objectUrl) {
        URL.revokeObjectURL(objectUrl);
      }
    };
  }, [objectUrl]);

  return {
    ...query,
    data: objectUrl,
  } as unknown as UseQueryResult<string | undefined, HttpError>;
}
