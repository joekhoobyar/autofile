import type { DocumentListParams } from '../models/document';

const DOCUMENT_LIST_PAGE_SIZES = new Set([6, 12, 24, 48, 96]);

export const DEFAULT_DOCUMENT_LIST_PARAMS: DocumentListParams = {
  per_page: 12,
  page: 1,
  sf: 'created_at',
  sd: true,
};

function parsePositiveIntParam(value: string | null): number | undefined {
  if (!value) return undefined;

  const parsed = Number.parseInt(value, 10);
  return Number.isInteger(parsed) && parsed > 0 ? parsed : undefined;
}

function parseBooleanParam(value: string | null): boolean | undefined {
  if (!value) return undefined;
  if (value === 'true' || value === 'desc') return true;
  if (value === 'false' || value === 'asc') return false;
  return undefined;
}

export function parseDocumentListHash(hash: string): DocumentListParams {
  const params = new URLSearchParams(hash.startsWith('#') ? hash.slice(1) : hash);
  const page = parsePositiveIntParam(params.get('page'));
  const perPage = parsePositiveIntParam(params.get('per_page'));
  const basicSearch = params.get('search')?.trim() || undefined;
  const matchAny = parseBooleanParam(params.get('match_any'));
  const titleSearch = params.get('q')?.trim() || undefined;
  const textSearch = params.get('text')?.trim() || undefined;
  const metadataValue = params.get('metadata_value')?.trim() || undefined;
  const filename = params.get('filename')?.trim() || undefined;
  const fileContentType = params.get('file_content_type')?.trim() || undefined;
  const documentTypeId = parsePositiveIntParam(params.get('document_type_id'));
  const metadataTypeId = parsePositiveIntParam(params.get('metadata_type_id'));
  const cabinetId = parsePositiveIntParam(params.get('cabinet_id'));
  const tagId = parsePositiveIntParam(params.get('tag_id'));
  const documentIndexValueId = parsePositiveIntParam(params.get('document_index_value_id'));

  return {
    ...DEFAULT_DOCUMENT_LIST_PARAMS,
    ...(page ? { page } : {}),
    ...(perPage && DOCUMENT_LIST_PAGE_SIZES.has(perPage) ? { per_page: perPage } : {}),
    ...(params.has('sf') ? { sf: params.get('sf') || undefined } : {}),
    ...(params.has('sd') ? { sd: parseBooleanParam(params.get('sd')) } : {}),
    ...(basicSearch
      ? {
          match_any: true,
          q: basicSearch,
          text: basicSearch,
        }
      : {
          ...(matchAny ? { match_any: true } : {}),
          ...(titleSearch ? { q: titleSearch } : {}),
          ...(textSearch ? { text: textSearch } : {}),
          ...(filename ? { filename } : {}),
        }),
    ...(documentTypeId ? { document_type_id: documentTypeId } : {}),
    ...(metadataValue ? { metadata_value: metadataValue } : {}),
    ...(metadataTypeId ? { metadata_type_id: metadataTypeId } : {}),
    ...(fileContentType ? { file_content_type: fileContentType } : {}),
    ...(cabinetId ? { cabinet_id: cabinetId } : {}),
    ...(tagId ? { tag_id: tagId } : {}),
    ...(documentIndexValueId ? { document_index_value_id: documentIndexValueId } : {}),
    ...(parseBooleanParam(params.get('duplicates')) ? { duplicates: true } : {}),
    ...(parseBooleanParam(params.get('duplicate_checksum')) ? { duplicate_checksum: true } : {}),
  };
}

export function parseBasicDocumentSearchHash(hash: string): string {
  const params = new URLSearchParams(hash.startsWith('#') ? hash.slice(1) : hash);
  return params.get('search')?.trim() ?? '';
}

function serializeDocumentListCommonParams(params: DocumentListParams, urlParams: URLSearchParams) {
  if (params.page && params.page !== DEFAULT_DOCUMENT_LIST_PARAMS.page) {
    urlParams.set('page', String(params.page));
  }
  if (params.per_page && params.per_page !== DEFAULT_DOCUMENT_LIST_PARAMS.per_page) {
    urlParams.set('per_page', String(params.per_page));
  }
  if (params.sf !== DEFAULT_DOCUMENT_LIST_PARAMS.sf) {
    urlParams.set('sf', params.sf ?? '');
  }
  if (params.sd !== DEFAULT_DOCUMENT_LIST_PARAMS.sd) {
    urlParams.set('sd', params.sd ? 'desc' : 'asc');
  }
}

export function serializeBasicDocumentSearchHash(value: string, params: DocumentListParams): string {
  const urlParams = new URLSearchParams();
  const searchText = value.trim();

  serializeDocumentListCommonParams({ ...params, page: 1 }, urlParams);
  if (params.document_type_id) {
    urlParams.set('document_type_id', String(params.document_type_id));
  }
  if (params.metadata_value?.trim()) {
    urlParams.set('metadata_value', params.metadata_value.trim());
  }
  if (params.metadata_type_id) {
    urlParams.set('metadata_type_id', String(params.metadata_type_id));
  }
  if (params.file_content_type?.trim()) {
    urlParams.set('file_content_type', params.file_content_type.trim());
  }
  if (params.cabinet_id) {
    urlParams.set('cabinet_id', String(params.cabinet_id));
  }
  if (params.tag_id) {
    urlParams.set('tag_id', String(params.tag_id));
  }
  if (params.document_index_value_id) {
    urlParams.set('document_index_value_id', String(params.document_index_value_id));
  }
  if (params.duplicates) {
    urlParams.set('duplicates', 'true');
  }
  if (params.duplicate_checksum) {
    urlParams.set('duplicate_checksum', 'true');
  }
  if (searchText) {
    urlParams.set('search', searchText);
  }

  return urlParams.toString();
}

export function serializeDocumentListUpdate(nextParams: DocumentListParams, appliedSearchText: string): string {
  const preservesBasicSearch =
    !!appliedSearchText &&
    nextParams.match_any &&
    nextParams.q === appliedSearchText &&
    nextParams.text === appliedSearchText;
  return preservesBasicSearch
    ? serializeBasicDocumentSearchHash(appliedSearchText, nextParams)
    : serializeDocumentListHash(nextParams);
}

export function serializeDocumentListHash(params: DocumentListParams): string {
  const urlParams = new URLSearchParams();

  serializeDocumentListCommonParams(params, urlParams);
  if (params.match_any) {
    urlParams.set('match_any', 'true');
  }
  if (params.q?.trim()) {
    urlParams.set('q', params.q.trim());
  }
  if (params.text?.trim()) {
    urlParams.set('text', params.text.trim());
  }
  if (params.document_type_id) {
    urlParams.set('document_type_id', String(params.document_type_id));
  }
  if (params.metadata_value?.trim()) {
    urlParams.set('metadata_value', params.metadata_value.trim());
  }
  if (params.metadata_type_id) {
    urlParams.set('metadata_type_id', String(params.metadata_type_id));
  }
  if (params.filename?.trim()) {
    urlParams.set('filename', params.filename.trim());
  }
  if (params.file_content_type?.trim()) {
    urlParams.set('file_content_type', params.file_content_type.trim());
  }
  if (params.cabinet_id) {
    urlParams.set('cabinet_id', String(params.cabinet_id));
  }
  if (params.tag_id) {
    urlParams.set('tag_id', String(params.tag_id));
  }
  if (params.document_index_value_id) {
    urlParams.set('document_index_value_id', String(params.document_index_value_id));
  }
  if (params.duplicates) {
    urlParams.set('duplicates', 'true');
  }
  if (params.duplicate_checksum) {
    urlParams.set('duplicate_checksum', 'true');
  }

  return urlParams.toString();
}
