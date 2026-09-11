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

export type DocumentListRouteIds = {
  tagId?: number;
  cabinetId?: number;
  documentIndexValueId?: number;
};

export type AdvancedDocumentSearchFormValues = {
  match_any: boolean;
  q: string;
  text: string;
  document_type_id: number | null;
  metadata_value: string;
  metadata_type_id: number | null;
  filename: string;
  file_content_type: string;
  cabinet_id: number | null;
  tag_id: number | null;
  duplicates: boolean;
  duplicate_checksum: boolean;
};

export type ActiveFilterChip = {
  key: string;
  label: string;
};

export type ChipRemoveResult =
  | { action: 'clear-search' }
  | { action: 'update'; params: DocumentListParams }
  | { action: 'navigate-documents' };

export function buildEffectiveListParams(
  listParams: DocumentListParams,
  routeIds: DocumentListRouteIds,
): DocumentListParams {
  return {
    ...listParams,
    tag_id: routeIds.tagId ?? listParams.tag_id,
    cabinet_id: routeIds.cabinetId ?? listParams.cabinet_id,
    document_index_value_id: routeIds.documentIndexValueId ?? listParams.document_index_value_id,
  };
}

export function buildAdvancedSearchParams(
  listParams: DocumentListParams,
  routeIds: DocumentListRouteIds,
): DocumentListParams {
  return buildEffectiveListParams(listParams, routeIds);
}

export function resolveSortValue(sf: string | undefined, sd: boolean | undefined): string | undefined {
  if (!sf) return undefined;
  return `${sf}:${sd ? 'desc' : 'asc'}`;
}

export function parseSortValue(value: string | undefined): { sf: string | undefined; sd: boolean | undefined } {
  if (!value) return { sf: undefined, sd: undefined };
  const [field, direction] = value.split(':');
  if (!field) return { sf: undefined, sd: undefined };
  return { sf: field, sd: direction === 'desc' };
}

export function resolveSearchText(draft: { appliedSearchText: string; value: string }, appliedSearchText: string): string {
  return draft.appliedSearchText === appliedSearchText ? draft.value : appliedSearchText;
}

export function advancedFormDefaults(
  existingParams: DocumentListParams,
  basicSearchText: string,
): AdvancedDocumentSearchFormValues {
  const hasBasicSearch = basicSearchText.length > 0;
  return {
    match_any: !!existingParams.match_any,
    q: hasBasicSearch ? '' : (existingParams.q ?? ''),
    text: hasBasicSearch ? '' : (existingParams.text ?? ''),
    document_type_id: existingParams.document_type_id ?? null,
    metadata_value: hasBasicSearch ? '' : (existingParams.metadata_value ?? ''),
    metadata_type_id: existingParams.metadata_type_id ?? null,
    filename: existingParams.filename ?? '',
    file_content_type: existingParams.file_content_type ?? '',
    cabinet_id: existingParams.cabinet_id ?? null,
    tag_id: existingParams.tag_id ?? null,
    duplicates: !!existingParams.duplicates,
    duplicate_checksum: !!existingParams.duplicate_checksum,
  };
}

export function advancedResetValues(): AdvancedDocumentSearchFormValues {
  return {
    match_any: false,
    q: '',
    text: '',
    document_type_id: null,
    metadata_value: '',
    metadata_type_id: null,
    filename: '',
    file_content_type: '',
    cabinet_id: null,
    tag_id: null,
    duplicates: false,
    duplicate_checksum: false,
  };
}

export function advancedSubmitParams(
  values: AdvancedDocumentSearchFormValues,
  existingParams: DocumentListParams,
): DocumentListParams {
  return {
    page: 1,
    per_page: existingParams.per_page,
    sf: existingParams.sf,
    sd: existingParams.sd,
    match_any: values.match_any || undefined,
    q: values.q.trim() || undefined,
    text: values.text.trim() || undefined,
    document_type_id: values.document_type_id ?? undefined,
    metadata_value: values.metadata_value.trim() || undefined,
    metadata_type_id: values.metadata_type_id ?? undefined,
    filename: values.filename.trim() || undefined,
    file_content_type: values.file_content_type.trim() || undefined,
    cabinet_id: values.cabinet_id ?? undefined,
    tag_id: values.tag_id ?? undefined,
    document_index_value_id: existingParams.document_index_value_id,
    duplicates: values.duplicates || undefined,
    duplicate_checksum: values.duplicate_checksum || undefined,
  };
}

export function clearSearchParams(listParams: DocumentListParams): DocumentListParams {
  return {
    ...listParams,
    match_any: undefined,
    q: undefined,
    text: undefined,
    page: 1,
  };
}

export function nextParamsForPage(
  listParams: DocumentListParams,
  page: number,
  perPage: number,
): DocumentListParams {
  return { ...listParams, page, per_page: perPage };
}

export function nextParamsForSort(
  listParams: DocumentListParams,
  sortValue: string | undefined,
): DocumentListParams {
  const { sf, sd } = parseSortValue(sortValue);
  return { ...listParams, sf, sd, page: 1 };
}

export function nextParamsForQuickFilter<T>(
  listParams: DocumentListParams,
  field: 'tag_id' | 'cabinet_id' | 'document_type_id',
  value: T | null | undefined,
): DocumentListParams {
  return { ...listParams, [field]: value ?? undefined, page: 1 };
}

export function buildActiveFilterChips(args: {
  listParams: DocumentListParams;
  appliedSearchText: string;
  routeIds: DocumentListRouteIds;
  documentTypeName?: string;
  metadataTypeName?: string;
  tagName?: string;
  cabinetName?: string;
}): ActiveFilterChip[] {
  const { listParams, appliedSearchText, routeIds, documentTypeName, metadataTypeName, tagName, cabinetName } = args;
  const chips: ActiveFilterChip[] = [];
  if (appliedSearchText) {
    chips.push({ key: 'basic-search', label: `Search: ${appliedSearchText}` });
  } else {
    if (listParams.q) chips.push({ key: 'q', label: `Title: ${listParams.q}` });
    if (listParams.text) chips.push({ key: 'text', label: `Text: ${listParams.text}` });
    if (listParams.filename) chips.push({ key: 'filename', label: `Filename: ${listParams.filename}` });
  }
  if (listParams.document_type_id) {
    chips.push({ key: 'document-type', label: `Document type: ${documentTypeName ?? listParams.document_type_id}` });
  }
  if (listParams.metadata_value) {
    chips.push({ key: 'metadata-value', label: `Metadata: ${listParams.metadata_value}` });
  }
  if (listParams.metadata_type_id) {
    chips.push({ key: 'metadata-type', label: `Metadata type: ${metadataTypeName ?? listParams.metadata_type_id}` });
  }
  if (listParams.file_content_type) {
    chips.push({ key: 'file-content-type', label: `Content type: ${listParams.file_content_type}` });
  }
  const effectiveTagId = routeIds.tagId ?? listParams.tag_id;
  const effectiveCabinetId = routeIds.cabinetId ?? listParams.cabinet_id;
  if (effectiveTagId && tagName) {
    chips.push({ key: routeIds.tagId ? `route-tag-${effectiveTagId}` : 'tag', label: `🏷️ ${tagName}` });
  }
  if (effectiveCabinetId && cabinetName) {
    chips.push({ key: routeIds.cabinetId ? `route-cabinet-${effectiveCabinetId}` : 'cabinet', label: `🗄️ ${cabinetName}` });
  }
  if (listParams.duplicates) chips.push({ key: 'duplicates', label: 'Duplicate title' });
  if (listParams.duplicate_checksum) chips.push({ key: 'duplicate-checksum', label: 'Duplicate file checksum' });
  return chips;
}

export function chipRemoveResult(chipKey: string, listParams: DocumentListParams): ChipRemoveResult {
  switch (chipKey) {
    case 'basic-search':
      return { action: 'clear-search' };
    case 'q':
      return { action: 'update', params: { ...listParams, q: undefined, page: 1 } };
    case 'text':
      return { action: 'update', params: { ...listParams, text: undefined, page: 1 } };
    case 'document-type':
      return { action: 'update', params: { ...listParams, document_type_id: undefined, page: 1 } };
    case 'metadata-value':
      return { action: 'update', params: { ...listParams, metadata_value: undefined, page: 1 } };
    case 'metadata-type':
      return { action: 'update', params: { ...listParams, metadata_type_id: undefined, page: 1 } };
    case 'filename':
      return { action: 'update', params: { ...listParams, filename: undefined, page: 1 } };
    case 'file-content-type':
      return { action: 'update', params: { ...listParams, file_content_type: undefined, page: 1 } };
    case 'tag':
      return { action: 'update', params: { ...listParams, tag_id: undefined, page: 1 } };
    case 'cabinet':
      return { action: 'update', params: { ...listParams, cabinet_id: undefined, page: 1 } };
    case 'duplicates':
      return { action: 'update', params: { ...listParams, duplicates: undefined, page: 1 } };
    case 'duplicate-checksum':
      return { action: 'update', params: { ...listParams, duplicate_checksum: undefined, page: 1 } };
    default:
      return { action: 'navigate-documents' };
  }
}

export function paginatorReportText(first: number, last: number, totalRecords: number): string {
  if (last != totalRecords) return `${first} - ${last} of ${totalRecords} documents`;
  if (totalRecords == 1) return ' 1 document ';
  return ` ${totalRecords} documents `;
}

export function toggleSelectedId(prev: Set<number>, id: number, checked: boolean): Set<number> {
  const next = new Set(prev);
  if (checked) next.add(id);
  else next.delete(id);
  return next;
}

export function toggleAllVisibleSelected(prev: Set<number>, visibleIds: number[], checked: boolean): Set<number> {
  const next = new Set(prev);
  for (const id of visibleIds) {
    if (checked) next.add(id);
    else next.delete(id);
  }
  return next;
}

export function isAllVisibleSelected(visibleIds: number[], selected: Set<number>): boolean {
  return visibleIds.length > 0 && visibleIds.every((id) => selected.has(id));
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
