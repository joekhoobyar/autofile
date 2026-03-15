import { useMutation, useQuery, useQueryClient, type UseMutationResult, type UseQueryResult } from '@tanstack/react-query';
import { apiFetchList, type ListParams, type ResourceList, type ResourceInput, HttpError, apiFetch, apiFetchRaw, apiMutate } from '../api';
import { type CabinetDocument, type Document, type DocumentMetadata, type NewCabinetDocument, type NewDocumentMetadata } from '../models/document';
import { useEffect, useMemo } from 'react';

export type DocumentTypeInput = ResourceInput<DocumentType>;

export interface CabinetDocumentInput {
  cabinet_id: number;
  documents: NewCabinetDocument[];
}

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

export function useSaveDocumentMetadata(id: string | number): UseMutationResult<DocumentMetadata, HttpError, NewDocumentMetadata[]> {
  const qc = useQueryClient();

  return useMutation<DocumentMetadata, HttpError, NewDocumentMetadata[]>({
    mutationFn: async (input) => {
      return apiMutate<DocumentMetadata, NewDocumentMetadata[]>(`api/v1/documents/${id}/metadata`, {
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
