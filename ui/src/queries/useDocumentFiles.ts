import { useQuery, type UseQueryResult } from '@tanstack/react-query';
import { apiFetch, apiFetchRaw, HttpError, parseApiError } from '../api';
import { type DocumentFile, type DocumentFileOcrPage, type DocumentFilePage } from '../models/documentFile';
import { useBlobObjectUrl } from './blobUrl';

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

export function useDocumentFileOcrPages(
  documentId: string | number,
  documentFileId: string | number
): UseQueryResult<DocumentFileOcrPage[], HttpError> {
  return useQuery({
    queryKey: ['documentFileOcrPage', 'list', documentId, documentFileId],
    enabled: !!documentId && !!documentFileId,
    queryFn: () => apiFetch<DocumentFileOcrPage[]>(`api/v1/documents/${documentId}/files/${documentFileId}/ocr-pages`),
  });
}

export function useDocumentFilePageImage(
  documentId: string | number,
  documentFileId: string | number,
  pageNumber: number,
  options = {}
): UseQueryResult<string | undefined, HttpError> {
  const query = useQuery<Blob | null, HttpError>({
    queryKey: ['documentFilePageImage', 'get', documentId, documentFileId, pageNumber],
    enabled: !!documentId && !!documentFileId && !!pageNumber,
    ...options,
    queryFn: async () => {
      const res = await apiFetchRaw(
        `api/v1/documents/${documentId}/files/${documentFileId}/pages/${pageNumber}/image`
      );
      if (res.status === 404) {
        return null;
      }
      if (!res.ok) {
        throw await parseApiError(res);
      }
      return await res.blob();
    },
  });

  const objectUrl = useBlobObjectUrl(query.data);

  return {
    ...query,
    data: objectUrl,
  } as unknown as UseQueryResult<string | undefined, HttpError>;
}
