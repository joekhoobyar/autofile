import { useMutation, useQuery, useQueryClient, type UseMutationResult, type UseQueryResult } from '@tanstack/react-query';
import { apiFetchList, type ResourceList, type ResourceInput, HttpError, apiFetch, apiFetchRaw, apiMutate, parseApiError } from '../api';
import { type CabinetDocument, type Document, type DocumentListParams, type DocumentMetadata, type NewCabinetDocument, type NewDocumentMetadata, type NewTagDocument, type TagDocument } from '../models/document';
import { useBlobObjectUrl } from './blobUrl';

export type DocumentTypeInput = ResourceInput<DocumentType>;

export interface TagDocumentInput {
  tag_id: number;
  documents: NewTagDocument[];
}

export interface RemoveTagDocumentInput {
  tag_id: number;
  documents: number[];
}

export interface CabinetDocumentInput {
  cabinet_id: number;
  documents: NewCabinetDocument[];
}

export interface RemoveCabinetDocumentInput {
  cabinet_id: number;
  documents: number[];
}

export function useDocuments(params: DocumentListParams): UseQueryResult<ResourceList<Document>, HttpError> {
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
  const query = useQuery<Blob | undefined, HttpError>({
    queryKey: ['document', 'get', {id}, 'thumbnail'],
    enabled: !!id,
    ...options,
    queryFn: async () => {
      const res = await apiFetchRaw(`api/v1/documents/${id}/thumbnail`);
      if (res.status === 404) {
        return undefined;
      }
      if (!res.ok) {
        throw await parseApiError(res);
      }
      return await res.blob();
    }
  });

  const objectUrl = useBlobObjectUrl(query.data);

  return {
    ...query,
    data: objectUrl,
  } as unknown as UseQueryResult<string | undefined, HttpError>;
}

export function useSaveDocumentMetadata(id: string | number): UseMutationResult<DocumentMetadata[], HttpError, NewDocumentMetadata[]> {
  const qc = useQueryClient();

  return useMutation<DocumentMetadata[], HttpError, NewDocumentMetadata[]>({
    mutationFn: async (input) => {
      return apiMutate<DocumentMetadata[], NewDocumentMetadata[]>(`api/v1/documents/${id}/metadata`, {
        method: "POST",
        body: input,
      });
    },

    onSuccess: () => {
      // invalidate by prefix (works with table params in the queryKey)
      qc.invalidateQueries({ queryKey: ["document"] });
    },
  });
}

export function useDeleteDocument(): UseMutationResult<void, HttpError, number> {
  const qc = useQueryClient();

  return useMutation<void, HttpError, number>({
    mutationFn: async (input) => {
      return apiMutate<void, void>(`api/v1/documents/${input}`, {
        method: "DELETE",
      });
    },

    onSuccess: () => {
      // invalidate by prefix (works with table params in the queryKey)
      qc.invalidateQueries({ queryKey: ["document"] });
    },
  });
}

export function useSaveCabinetDocument(): UseMutationResult<CabinetDocument[], HttpError, CabinetDocumentInput> {
  const qc = useQueryClient();

  return useMutation<CabinetDocument[], HttpError, CabinetDocumentInput>({
    mutationFn: async (input) => {
      return apiMutate<CabinetDocument[], NewCabinetDocument[]>(`api/v1/cabinets/${input.cabinet_id}/documents`, {
        method: "POST",
        body: input.documents,
      });
    },

    onSuccess: () => {
      // invalidate by prefix (works with table params in the queryKey)
      qc.invalidateQueries({ queryKey: ["document"] });
    },
  });
}

export function useRemoveCabinetDocument(): UseMutationResult<CabinetDocument[], HttpError, RemoveCabinetDocumentInput> {
  const qc = useQueryClient();

  return useMutation<CabinetDocument[], HttpError, RemoveCabinetDocumentInput>({
    mutationFn: async (input) => {
      return apiMutate<CabinetDocument[], number[]>(`api/v1/cabinets/${input.cabinet_id}/documents/delete`, {
        method: "POST",
        body: input.documents,
      });
    },

    onSuccess: () => {
      // invalidate by prefix (works with table params in the queryKey)
      qc.invalidateQueries({ queryKey: ["document"] });
    },
  });
}

export function useSaveTagDocument(): UseMutationResult<TagDocument[], HttpError, TagDocumentInput> {
  const qc = useQueryClient();

  return useMutation<TagDocument[], HttpError, TagDocumentInput>({
    mutationFn: async (input) => {
      return apiMutate<TagDocument[], NewTagDocument[]>(`api/v1/tags/${input.tag_id}/documents`, {
        method: "POST",
        body: input.documents,
      });
    },

    onSuccess: () => {
      // invalidate by prefix (works with table params in the queryKey)
      qc.invalidateQueries({ queryKey: ["document"] });
    },
  });
}

export function useRemoveTagDocument(): UseMutationResult<TagDocument[], HttpError, RemoveTagDocumentInput> {
  const qc = useQueryClient();

  return useMutation<TagDocument[], HttpError, RemoveTagDocumentInput>({
    mutationFn: async (input) => {
      return apiMutate<TagDocument[], number[]>(`api/v1/tags/${input.tag_id}/documents/delete`, {
        method: "POST",
        body: input.documents,
      });
    },

    onSuccess: () => {
      // invalidate by prefix (works with table params in the queryKey)
      qc.invalidateQueries({ queryKey: ["document"] });
    },
  });
}
