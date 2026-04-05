import { useQuery, type UseQueryResult } from '@tanstack/react-query';
import { apiFetch, HttpError } from '../api';
import { type DocumentFile, type DocumentFilePage } from '../models/documentFile';

export function useDocumentFiles(documentId: string | number): UseQueryResult<DocumentFile[], HttpError> {
  return useQuery({
    queryKey: ['documentFile', 'list', documentId],
    enabled: !!documentId,
    queryFn: () => apiFetch<DocumentFile[]>(`api/v1/documents/${documentId}/files`),
  });
}

export function useDocumentFilePages(
  documentId: string | number,
  documentFileId: string | number
): UseQueryResult<DocumentFilePage[], HttpError> {
  return useQuery({
    queryKey: ['documentFilePage', 'list', documentId, documentFileId],
    enabled: !!documentId && !!documentFileId,
    queryFn: () => apiFetch<DocumentFilePage[]>(`api/v1/documents/${documentId}/files/${documentFileId}/pages`),
  });
}
