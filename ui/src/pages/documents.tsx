import { useMemo, useRef, useState, type ReactNode } from 'react';
import { Link, useLocation, useNavigate, useParams } from 'react-router-dom';
import { Controller, useForm } from 'react-hook-form';

import { Card } from 'primereact/card';
import { Checkbox } from 'primereact/checkbox';
import { DataView, DataViewLayoutOptions, type DataViewPageEvent } from 'primereact/dataview';
import { Dropdown } from 'primereact/dropdown';
import { Dialog } from 'primereact/dialog';
import { classNames } from 'primereact/utils';
import { format } from "date-fns";

import { apiUrl, getAccessToken } from '../api';
import { useDocuments, useDocumentThumbnail } from '../queries/useDocuments';
import { useDocumentIndex } from '../queries/useDocumentIndexes';
import { useDocumentIndexValueAncestors } from '../queries/useDocumentIndexValues';
import { type Document, type DocumentListParams } from '../models/document';
import { useMetadataTypes, useMetadataTypesMap } from '../queries/useMetadataTypes';
import { useDocumentTypes, useDocumentTypesMap } from '../queries/useDocumentTypes';
import { Menu } from 'primereact/menu';
import type { MenuItem } from 'primereact/menuitem';
import { Button } from 'primereact/button';
import { useCabinets } from '../queries/useCabinets';
import { MAX_CABINETS, type Cabinet } from '../models/cabinet';
import { useTags } from '../queries/useTags';
import type { Tag as TagModel } from '../models/tag';
import { Toast } from 'primereact/toast';
import { Tooltip } from 'primereact/tooltip';
import { FileUpload, type FileUploadFile, type FileUploadHandlerEvent, type FileUploadSelectEvent, type FileUploadUploadEvent, type ItemTemplateOptions } from 'primereact/fileupload';
import { ProgressBar } from 'primereact/progressbar';
import { Tag } from 'primereact/tag';
import { InputText } from 'primereact/inputtext';
import { Badge } from 'primereact/badge';
import { Chip } from 'primereact/chip';
import { DocumentActions } from '../components/DocumentActions';
import { DocumentViewLayout } from '../components/DocumentViewLayout';
import { Message } from 'primereact/message';
import { useId } from '../util';
import { useDocument, useSaveDocument } from '../queries/useDocuments';

type DocumentListItemProps = {
  doc: Readonly<Document>;
  index: number;
  onImageClick: (src: string | undefined, title: string) => void;
  selected: boolean;
  onSelectionChange: (id: number, checked: boolean) => void;
  cabinetLookup: Record<number, Cabinet>;
  tagLookup: Record<number, TagModel>;
};

type DocumentThumbnailProps = {
  src: string | undefined;
  alt: string;
  onClick: (src: string | undefined) => void;
  imgClassName: string;
  placeholderClassName: string;
  buttonStyle?: React.CSSProperties;
};

const DOCUMENT_LIST_PAGE_SIZES = [6, 12, 24, 48, 96];

const DEFAULT_DOCUMENT_LIST_PARAMS: DocumentListParams = {
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

function parseDocumentListHash(hash: string): DocumentListParams {
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

  return {
    ...DEFAULT_DOCUMENT_LIST_PARAMS,
    ...(page ? { page } : {}),
    ...(perPage && DOCUMENT_LIST_PAGE_SIZES.includes(perPage) ? { per_page: perPage } : {}),
    ...(params.has('sf') ? { sf: params.get('sf') || undefined } : {}),
    ...(params.has('sd') ? { sd: parseBooleanParam(params.get('sd')) } : {}),
    ...(basicSearch ? {
      match_any: true,
      q: basicSearch,
      text: basicSearch,
      metadata_value: basicSearch,
    } : {
      ...(matchAny ? { match_any: true } : {}),
      ...(titleSearch ? { q: titleSearch } : {}),
      ...(textSearch ? { text: textSearch } : {}),
      ...(documentTypeId ? { document_type_id: documentTypeId } : {}),
      ...(metadataValue ? { metadata_value: metadataValue } : {}),
      ...(metadataTypeId ? { metadata_type_id: metadataTypeId } : {}),
      ...(filename ? { filename } : {}),
      ...(fileContentType ? { file_content_type: fileContentType } : {}),
      ...(cabinetId ? { cabinet_id: cabinetId } : {}),
      ...(tagId ? { tag_id: tagId } : {}),
    }),
    ...(parseBooleanParam(params.get('duplicates')) ? { duplicates: true } : {}),
    ...(parseBooleanParam(params.get('duplicate_checksum')) ? { duplicate_checksum: true } : {}),
  };
}

function parseBasicDocumentSearchHash(hash: string): string {
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

function serializeBasicDocumentSearchHash(value: string, params: DocumentListParams): string {
  const urlParams = new URLSearchParams();
  const searchText = value.trim();

  serializeDocumentListCommonParams({ ...params, page: 1 }, urlParams);
  if (searchText) {
    urlParams.set('search', searchText);
  }

  return urlParams.toString();
}

function serializeDocumentListHash(params: DocumentListParams): string {
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
  if (params.duplicates) {
    urlParams.set('duplicates', 'true');
  }
  if (params.duplicate_checksum) {
    urlParams.set('duplicate_checksum', 'true');
  }

  return urlParams.toString();
}

function DocumentThumbnail({
  src,
  alt,
  onClick,
  imgClassName,
  placeholderClassName,
}: Readonly<DocumentThumbnailProps>) {
  const [loadedSrc, setLoadedSrc] = useState<string | undefined>(undefined);
  const [errorSrc, setErrorSrc] = useState<string | undefined>(undefined);

  const isLoaded = !!src && loadedSrc === src;
  const hasError = !!src && errorSrc === src;

  const handleLoad = () => {
    if (src) {
      setLoadedSrc(src);
      setErrorSrc(undefined);
    }
  };

  const handleError = () => {
    if (src) {
      setErrorSrc(src);
    }
  };

  return (
    <div className="aut-document-thumbnail-wrapper">
      <button type="button" onClick={() => onClick(src)}>
        {src && !hasError ? (
          <img
            alt={alt}
            className={imgClassName}
            src={src}
            onLoad={handleLoad}
            onError={handleError}
            style={{ maxHeight: '200px', visibility: isLoaded ? 'visible' : 'hidden' }}
          />
        ) : (
          <div
            className={placeholderClassName}
            style={{ maxHeight: '200px', height: '200px', display: 'flex', alignItems: 'center', justifyContent: 'center' }}
            aria-label="Loading thumbnail"
          >
            <span className="pi pi-spin pi-spinner" aria-hidden="true" />
          </div>
        )}
      </button>
    </div>
  );
}

/**
 * Renders a document in list layout for the DataView component.
 * 
 * @param doc document
 * @param index index in the list
 * @returns HTML element for a document in list layout
 */
function DocumentListItem({ doc, index, onImageClick, selected, onSelectionChange, cabinetLookup, tagLookup }: Readonly<DocumentListItemProps>) {
    const { data } = useDocumentThumbnail(doc.id);
    const { data: mdt } = useMetadataTypesMap('slug');
    const { data: ddt } = useDocumentTypesMap();
    const cabinetItems = (doc.cabinet_ids ?? [])
      .map((id) => cabinetLookup[id])
      .filter((cabinet): cabinet is Cabinet => !!cabinet)
      .sort((a, b) => (a.displayName ?? a.name ?? a.slug).localeCompare(b.displayName ?? b.name ?? b.slug));
    const tagItems = (doc.tag_ids ?? [])
      .map((id) => tagLookup[id])
      .filter((tag): tag is TagModel => !!tag)
      .sort((a, b) => a.name.localeCompare(b.name));

    return (
      <div className={classNames('col-12 aut-document-list', { 'is-selected': selected })} key={doc.id}>
        <div className={classNames('flex flex-column xl:flex-row xl:align-items-start p-4 gap-4', { 'border-top-1 surface-border': index !== 0 })}>
          <DocumentThumbnail
            src={data}
            alt="Page 1"
            onClick={(src) => onImageClick(src, doc.title)}
            imgClassName="w-9 sm:w-16rem xl:w-10rem block xl:block mx-auto aut-document-thumbnail"
            placeholderClassName="w-9 sm:w-16rem xl:w-10rem block xl:block mx-auto aut-document-thumbnail"
          />
          <section className="flex flex-column sm:flex-row justify-content-between align-items-center xl:align-items-start flex-1 gap-4 aut-document">
            <div className="flex flex-column align-items-center sm:align-items-start gap-3 aut-document-header">
              <header className="flex align-items-center gap-2 aut-document-header-row">
                <Checkbox
                  inputId={`document-select-${doc.id}`}
                  onChange={(event) => onSelectionChange(doc.id, !!event.checked)}
                  checked={selected}
                />
                <Link to={`/documents/${doc.id}/preview`}>{doc.title}</Link>
              </header>
            </div>
            <aside className="flex flex-column align-items-center sm:align-items-start">
              <ul className="aut-document-metadata">
                <li><span>Type</span>: {ddt?.[doc.document_type_id]?.name}</li>
                <li><span>Pages</span>: {doc.pages ?? 0}</li>
                <li><span>Created</span>: {format(new Date(doc.created_at), "MM/dd/yyyy HH:mm")}</li>
                {Object.entries(doc.metadata)
                  .sort(([keyA], [keyB]) => {
                    const nameA = mdt?.[keyA]?.name ?? keyA;
                    const nameB = mdt?.[keyB]?.name ?? keyB;
                    return nameA.localeCompare(nameB);
                  })
                  .map(([key, value]) => (
                    <li key={key}><span>{mdt?.[key].name ?? key}</span>: {value}</li>
                  ))}
              </ul>
            </aside>
            <aside className="flex flex-column align-items-center sm:align-items-start">
              {tagItems.length > 0 && (
                <ul className="aut-document-tags">
                  {tagItems.map((tag) => (
                    <li key={tag.id}>
                      <Link to={`/tags/${tag.id}/documents`}>
                        <Badge value={tag.name} className="aut-document-tag" style={{ backgroundColor: `#${tag.color}` }} />
                      </Link>
                    </li>
                  ))}
                </ul>
              )}
              {cabinetItems.length > 0 && (
                <ul className="aut-document-cabinets">
                  {cabinetItems.map((cabinet) => (
                    <li key={cabinet.id}>
                      <Link to={`/cabinets/${cabinet.id}/documents`}>
                        <Badge value={`🗄️ ${cabinet.displayName ?? cabinet.name ?? cabinet.slug}`} severity="secondary" />
                      </Link>
                    </li>
                  ))}
                </ul>
              )}
            </aside>
          </section>
        </div>
      </div>
    );
}

type DocumentGridItemProps = {
  doc: Readonly<Document>;
  onImageClick: (src: string | undefined, title: string) => void;
  selected: boolean;
  onSelectionChange: (id: number, checked: boolean) => void;
  cabinetLookup: Record<number, Cabinet>;
  tagLookup: Record<number, TagModel>;
};

/**
 * Renders a document in grid layout for the DataView component.
 * 
 * @param doc document
 * @returns HTML element for a document in grid layout
 */
function DocumentGridItem({ doc, onImageClick, selected, onSelectionChange, cabinetLookup, tagLookup }: Readonly<DocumentGridItemProps>) {
    const { data } = useDocumentThumbnail(doc.id);
    const { data: mdt } = useMetadataTypesMap('slug');
    const { data: ddt } = useDocumentTypesMap();
    const cabinetItems = (doc.cabinet_ids ?? [])
      .map((id) => cabinetLookup[id])
      .filter((cabinet): cabinet is Cabinet => !!cabinet)
      .sort((a, b) => (a.displayName ?? a.name ?? a.slug).localeCompare(b.displayName ?? b.name ?? b.slug));
    const tagItems = (doc.tag_ids ?? [])
      .map((id) => tagLookup[id])
      .filter((tag): tag is TagModel => !!tag)
      .sort((a, b) => a.name.localeCompare(b.name));

    const handleGridClick = (event: React.MouseEvent) => {
      const target = event.target as HTMLElement;
      if (target.closest('a, button, .p-checkbox')) return;
      onSelectionChange(doc.id, !selected);
    };

    return (
      <div className={classNames('col-12 sm:col-6 lg:col-4 xl:col-2 p-2 aut-document-grid', { 'is-selected': selected })} key={doc.id}>
        <div className="border-1 surface-border surface-card border-round" onClick={handleGridClick}>
          <section className="flex flex-column aut-document w-full">
            <header className="flex align-items-center gap-2 aut-document-header aut-document-header-row">
              <Checkbox
                inputId={`document-select-${doc.id}`}
                onChange={(event) => onSelectionChange(doc.id, !!event.checked)}
                checked={selected}
              />
              <Link to={`/documents/${doc.id}/preview`}>{doc.title}</Link>
            </header>
            <aside>
              <DocumentThumbnail
                src={data}
                alt="Page 1"
                onClick={(src) => onImageClick(src, doc.title)}
                imgClassName="aut-document-thumbnail"
                placeholderClassName="aut-document-thumbnail"
              />
              <ul className="aut-document-metadata">
                <li><span>Type</span>: {ddt?.[doc.document_type_id]?.name}</li>
                <li><span>Pages</span>: {doc.pages ?? 0}</li>
                <li><span>Created</span>: {format(new Date(doc.created_at), "MM/dd/yyyy HH:mm")}</li>
                {Object.entries(doc.metadata)
                  .sort(([keyA], [keyB]) => {
                    const nameA = mdt?.[keyA]?.name ?? keyA;
                    const nameB = mdt?.[keyB]?.name ?? keyB;
                    return nameA.localeCompare(nameB);
                  })
                  .map(([key, value]) => (
                    <li key={key}><span>{mdt?.[key].name ?? key}</span>: {value}</li>
                  ))}
              </ul>
              {tagItems.length > 0 && (
                <ul className="aut-document-tags">
                  {tagItems.map((tag) => (
                    <li key={tag.id}>
                      <Link to={`/tags/${tag.id}/documents`}>
                        <Badge value={tag.name} className="aut-document-tag" style={{ backgroundColor: `#${tag.color}` }} />
                      </Link>
                    </li>
                  ))}
                </ul>
              )}
              {cabinetItems.length > 0 && (
                <ul className="aut-document-cabinets">
                  {cabinetItems.map((cabinet) => (
                    <li key={cabinet.id}>
                      <Link to={`/cabinets/${cabinet.id}/documents`}>
                        <Badge value={`🗄️ ${cabinet.displayName ?? cabinet.name ?? cabinet.slug}`} severity="secondary" />
                      </Link>
                    </li>
                  ))}
                </ul>
              )}
            </aside>
          </section>
        </div>
      </div>
    );
}

/**
 * Renders a list or grid of documents.
 * 
 * @returns HTML element for a list or grid of documents
 */
export function ListDocuments() {
  const navigate = useNavigate();
  const location = useLocation();
  const params = useParams();
  const listParams = useMemo(() => parseDocumentListHash(location.hash), [location.hash]);
  const tagId = params.tagId ? Number.parseInt(params.tagId) : undefined;
  const cabinetId = params.cabinetId ? Number.parseInt(params.cabinetId) : undefined;
  const documentIndexValueId = params.documentIndexValueId ? Number.parseInt(params.documentIndexValueId) : undefined;
  const documentIndexId = params.documentIndexId ? Number.parseInt(params.documentIndexId) : undefined;
  const effectiveListParams: DocumentListParams = {
    ...listParams,
    tag_id: tagId ?? listParams.tag_id,
    cabinet_id: cabinetId ?? listParams.cabinet_id,
    document_index_value_id: documentIndexValueId ?? listParams.document_index_value_id,
  };
  const showIndexMenu = !!documentIndexId && !!documentIndexValueId;
  const { data: indexAncestors } = useDocumentIndexValueAncestors(documentIndexId ?? 0, documentIndexValueId ?? 0);
  const { data: documentIndex } = useDocumentIndex(documentIndexId ?? 0, { enabled: !!documentIndexId });
  const [layout, setLayout] = useState<'list' | 'grid'>('grid');
  const [previewVisible, setPreviewVisible] = useState(false);
  const [previewSrc, setPreviewSrc] = useState<string | undefined>(undefined);
  const [previewTitle, setPreviewTitle] = useState('');
  const [selectedIds, setSelectedIds] = useState<Set<number>>(new Set());
  const appliedSearchText = parseBasicDocumentSearchHash(location.hash);
  const [searchDraft, setSearchDraft] = useState({ appliedSearchText, value: appliedSearchText });
  const searchText = searchDraft.appliedSearchText === appliedSearchText ? searchDraft.value : appliedSearchText;
  const setSearchText = (value: string) => setSearchDraft({ appliedSearchText, value });
  const { isPending, data, isFetching } = useDocuments(effectiveListParams);
  const visibleDocumentIds = useMemo(() => data?.items.map((doc) => doc.id) ?? [], [data?.items]);
  const allVisibleSelected = visibleDocumentIds.length > 0 && visibleDocumentIds.every((id) => selectedIds.has(id));
  const { data: cabinetOptions } = useCabinets({ page: 1, per_page: MAX_CABINETS });
  const { data: tagOptions } = useTags({ page: 1, per_page: 200 });
  const { data: documentTypeLookup } = useDocumentTypesMap();
  const { data: metadataTypeLookup } = useMetadataTypesMap('id');
  const cabinetLookup = useMemo(() => {
    const lookup: Record<number, Cabinet> = {};
    for (const cabinet of cabinetOptions?.items ?? []) {
      lookup[cabinet.id] = cabinet;
    }
    return lookup;
  }, [cabinetOptions?.items]);
  const tagLookup = useMemo(() => {
    const lookup: Record<number, TagModel> = {};
    for (const tag of tagOptions?.items ?? []) {
      lookup[tag.id] = tag;
    }
    return lookup;
  }, [tagOptions?.items]);

  const activeFilterChips = useMemo(() => {
    const chips: Array<{ key: string; label: string }> = [];
    if (appliedSearchText) {
      chips.push({ key: 'basic-search', label: `Search: ${appliedSearchText}` });
    } else {
      if (listParams.q) {
        chips.push({ key: 'q', label: `Title: ${listParams.q}` });
      }
      if (listParams.text) {
        chips.push({ key: 'text', label: `Text: ${listParams.text}` });
      }
      if (listParams.document_type_id) {
        chips.push({
          key: 'document-type',
          label: `Document type: ${documentTypeLookup?.[String(listParams.document_type_id)]?.name ?? listParams.document_type_id}`,
        });
      }
      if (listParams.metadata_value) {
        chips.push({ key: 'metadata-value', label: `Metadata: ${listParams.metadata_value}` });
      }
      if (listParams.metadata_type_id) {
        chips.push({
          key: 'metadata-type',
          label: `Metadata type: ${metadataTypeLookup?.[String(listParams.metadata_type_id)]?.name ?? listParams.metadata_type_id}`,
        });
      }
      if (listParams.filename) {
        chips.push({ key: 'filename', label: `Filename: ${listParams.filename}` });
      }
      if (listParams.file_content_type) {
        chips.push({ key: 'file-content-type', label: `Content type: ${listParams.file_content_type}` });
      }
    }
    const effectiveTagId = tagId ?? listParams.tag_id;
    const effectiveCabinetId = cabinetId ?? listParams.cabinet_id;
    if (effectiveTagId && tagLookup[effectiveTagId]) {
      chips.push({
        key: tagId ? `route-tag-${effectiveTagId}` : 'tag',
        label: `🏷️ ${tagLookup[effectiveTagId].name}`,
      });
    }
    if (effectiveCabinetId && cabinetLookup[effectiveCabinetId]) {
      const cabinet = cabinetLookup[effectiveCabinetId];
      chips.push({
        key: cabinetId ? `route-cabinet-${effectiveCabinetId}` : 'cabinet',
        label: `🗄️ ${cabinet.displayName ?? cabinet.name ?? cabinet.slug}`,
      });
    }
    if (listParams.duplicates) {
      chips.push({ key: 'duplicates', label: 'Duplicate title' });
    }
    if (listParams.duplicate_checksum) {
      chips.push({ key: 'duplicate-checksum', label: 'Duplicate file checksum' });
    }
    return chips;
  }, [appliedSearchText, tagId, cabinetId, tagLookup, cabinetLookup, listParams, documentTypeLookup, metadataTypeLookup]);

  const sortOptions = [
    { label: 'ID (Ascending)', value: 'id:asc' },
    { label: 'ID (Descending)', value: 'id:desc' },
    { label: 'Title (A-Z)', value: 'title:asc' },
    { label: 'Title (Z-A)', value: 'title:desc' },
    { label: 'Created (Oldest)', value: 'created_at:asc' },
    { label: 'Created (Newest)', value: 'created_at:desc' },
  ];

  const sortDir = listParams.sd ? 'desc' : 'asc';
  const sortValue = listParams.sf ? `${listParams.sf}:${sortDir}` : undefined;

  const updateListParams = (nextParams: DocumentListParams) => {
    navigate({
      pathname: location.pathname,
      search: location.search,
      hash: serializeDocumentListHash(nextParams),
    });
  };

  const onPage = (event: DataViewPageEvent) => {
    updateListParams({ ...listParams, page: event.page + 1, per_page: event.rows });
  };

  const onSortChange = (value: string | undefined) => {
    if (!value) {
      updateListParams({ ...listParams, sf: undefined, sd: undefined, page: 1 });
      return;
    }

    const [field, direction] = value.split(':');
    updateListParams({
      ...listParams,
      sf: field,
      sd: direction === 'desc',
      page: 1,
    });
  };

  const applySearch = () => {
    navigate({
      pathname: location.pathname,
      search: location.search,
      hash: serializeBasicDocumentSearchHash(searchText, listParams),
    });
  };

  const clearSearch = () => {
    setSearchText('');
    updateListParams({
      ...listParams,
      match_any: undefined,
      q: undefined,
      text: undefined,
      document_type_id: undefined,
      metadata_value: undefined,
      metadata_type_id: undefined,
      filename: undefined,
      file_content_type: undefined,
      cabinet_id: undefined,
      tag_id: undefined,
      duplicates: undefined,
      duplicate_checksum: undefined,
      page: 1,
    });
  }

  const openPreview = (src: string | undefined, title: string) => {
    if (!src) return;
    setPreviewSrc(src);
    setPreviewTitle(title);
    setPreviewVisible(true);
  };

  const closePreview = () => {
    setPreviewVisible(false);
  };

  const handleSelectionChange = (id: number, checked: boolean) => {
    setSelectedIds((prev) => {
      const next = new Set(prev);
      if (checked) {
        next.add(id);
      } else {
        next.delete(id);
      }
      return next;
    });
  };

  const handleSelectAllChange = (checked: boolean) => {
    setSelectedIds((prev) => {
      const next = new Set(prev);
      for (const id of visibleDocumentIds) {
        if (checked) {
          next.add(id);
        } else {
          next.delete(id);
        }
      }
      return next;
    });
  };


  const itemTemplate = (doc: Document, layout: 'list' | 'grid', index: number) => {
    if (!doc)
      return;
    if (layout === 'list')
      return (
        <DocumentListItem
          key={doc.id}
          doc={doc}
          index={index}
          onImageClick={openPreview}
          selected={selectedIds.has(doc.id)}
          onSelectionChange={handleSelectionChange}
          cabinetLookup={cabinetLookup}
          tagLookup={tagLookup}
        />
      );
    else if (layout === 'grid')
      return (
        <DocumentGridItem
          key={doc.id}
          doc={doc}
          onImageClick={openPreview}
          selected={selectedIds.has(doc.id)}
          onSelectionChange={handleSelectionChange}
          cabinetLookup={cabinetLookup}
          tagLookup={tagLookup}
        />
      );
  };

  const listTemplate = (docs: Document[], layout: 'list' | 'grid') => {
      return <div className="grid grid-nogutter">{docs.map((doc, index) => itemTemplate(doc, layout, index))}</div>;
  };

  const header = () => {
    const advancedSearchParams: DocumentListParams = {
      ...listParams,
      cabinet_id: cabinetId ?? listParams.cabinet_id,
      tag_id: tagId ?? listParams.tag_id,
    };

    return (
      <div className="flex flex-column gap-3 md:flex-row md:justify-content-between md:align-items-center">
        <div className="flex flex-column gap-2 md:flex-row md:align-items-center w-full md:w-auto">
          <div className="p-inputgroup w-full md:w-30rem">
            <InputText
              value={searchText}
              onChange={(event) => setSearchText(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === 'Enter') {
                  event.preventDefault();
                  applySearch();
                }
              }}
              className="p-inputtext-sm"
              placeholder="Search title and document text"
              aria-label="Search documents"
            />
            {searchText && (
              <span className="p-inputgroup-addon p-0">
                <Button
                  type="button"
                  icon="pi pi-times"
                  aria-label="Clear search"
                  onClick={clearSearch}
                  size="small"
                  className="p-button-secondary h-full"
                  style={{ borderRadius: 0 }}
                />
              </span>
            )}
            <span className="p-inputgroup-addon p-0">
              <Button
                type="button"
                icon="pi pi-search"
                aria-label="Search"
                onClick={applySearch}
                size="small"
                className="p-button-info h-full"
                style={{ borderTopLeftRadius: 0, borderBottomLeftRadius: 0 }}
              />
            </span>
          </div>
          <Button
            type="button"
            label="Advanced Search"
            link
            size="small"
            onClick={() => navigate({
              pathname: '/documents/search',
              hash: serializeDocumentListHash(advancedSearchParams),
            })}
            className="align-self-start md:align-self-center p-0 md:ml-5"
          />
        </div>
        <div className="flex justify-content-end align-items-center gap-3">
          <div className="flex align-items-center gap-2">
            <Checkbox
              inputId="documents-select-all"
              checked={allVisibleSelected}
              onChange={(event) => handleSelectAllChange(!!event.checked)}
              disabled={visibleDocumentIds.length === 0}
              aria-label="Select all documents on this page"
            />
            <label htmlFor="documents-select-all">Select all</label>
          </div>
          <DataViewLayoutOptions layout={layout} onChange={(e) => setLayout(e.value as 'list' | 'grid')} />
        </div>
      </div>
    );
  };

  const paginatorTemplate = {
    layout: 'RowsPerPageDropdown PrevPageLink PageLinks NextPageLink CurrentPageReport',
    CurrentPageReport: (options: { first: number; last: number; totalRecords: number }) => (
      <div className="flex align-items-center gap-3 aut-documents-paginator-report">
        <Dropdown
          value={sortValue}
          options={sortOptions}
          placeholder="Sort by"
          onChange={(event) => onSortChange(event.value as string | undefined)}
          className="w-15rem"
          aria-label="Sort documents"
        />
        {activeFilterChips.map((chip) => (
          <Chip
            key={chip.key}
            label={chip.label}
            removable
            onRemove={() => {
              switch (chip.key) {
                case 'basic-search':
                  clearSearch();
                  break;
                case 'q':
                  updateListParams({ ...listParams, q: undefined, page: 1 });
                  break;
                case 'text':
                  updateListParams({ ...listParams, text: undefined, page: 1 });
                  break;
                case 'document-type':
                  updateListParams({ ...listParams, document_type_id: undefined, page: 1 });
                  break;
                case 'metadata-value':
                  updateListParams({ ...listParams, metadata_value: undefined, page: 1 });
                  break;
                case 'metadata-type':
                  updateListParams({ ...listParams, metadata_type_id: undefined, page: 1 });
                  break;
                case 'filename':
                  updateListParams({ ...listParams, filename: undefined, page: 1 });
                  break;
                case 'file-content-type':
                  updateListParams({ ...listParams, file_content_type: undefined, page: 1 });
                  break;
                case 'tag':
                  updateListParams({ ...listParams, tag_id: undefined, page: 1 });
                  break;
                case 'cabinet':
                  updateListParams({ ...listParams, cabinet_id: undefined, page: 1 });
                  break;
                case 'duplicates':
                  updateListParams({ ...listParams, duplicates: undefined, page: 1 });
                  break;
                case 'duplicate-checksum':
                  updateListParams({ ...listParams, duplicate_checksum: undefined, page: 1 });
                  break;
                default:
                  navigate('/documents');
              }
              return true;
            }}
          />
        ))}
        <span className="aut-documents-paginator-count">
          {options.first} - {options.last} of {options.totalRecords}
        </span>
      </div>
    ),
  };

  const indexMenuItems = useMemo<MenuItem[]>(() => {
    if (!showIndexMenu) return [];
    const rootItem: MenuItem = {
      label: documentIndex?.name ?? 'Document Index',
      icon: 'pi pi-folder',
      command: () => navigate(`/indexes/${documentIndexId}/values`),
    };

    const stackItems = (indexAncestors ?? []).map((item, index, items) => {
      const suffix = index === items.length - 1 ? '/documents' : '';
      return {
        label: item.value,
        icon: 'pi pi-folder',
        command: () => navigate(`/indexes/${documentIndexId}/values/${item.id}${suffix}`),
      };
    });

    return [rootItem, ...stackItems];
  }, [documentIndex?.name, documentIndexId, indexAncestors, navigate, showIndexMenu]);

  const mainContent = (
    <>
    <DocumentActions
        documentIds={Array.from(selectedIds)}
        onAfterAction={() => setSelectedIds(new Set())}
        includeNewDocument
        buttonClassName="mt-3 mr-5"
        buttonStyle={{ float: 'right' }}
      />

    <Card title="Documents">
      <DataView className="aut-documents-data-view" value={data?.items ?? []}
          loading={isPending || isFetching}
          lazy
          emptyMessage="No documents match."
          onPage={onPage}
          paginator={true}
          first={Math.max(((data?.page ?? listParams.page ?? 1) - 1) * (data?.per_page ?? listParams.per_page ?? 0), 0)}
          rows={data?.per_page ?? listParams.per_page}
          totalRecords={data?.total}
          paginatorTemplate={paginatorTemplate}
          paginatorPosition="both"
          rowsPerPageOptions={[6, 12, 24, 48, 96]}
          listTemplate={listTemplate} layout={layout} header={header()}
        />
    </Card>
    <Dialog
      header={previewTitle || 'Document Preview'}
      visible={previewVisible}
      onHide={closePreview}
      style={{ width: '90vw', maxWidth: '900px' }}
      dismissableMask={true}
    >
      {previewSrc && (
        <img
          alt={previewTitle || 'Document image'}
          src={previewSrc}
          style={{ width: '100%', height: 'auto', display: 'block' }}
        />
      )}
    </Dialog>
    </>
  );

  if (!showIndexMenu) {
    return mainContent;
  }

  return (
    <div className="flex gap-3 align-items-start">
      <Menu model={indexMenuItems} style={{ minWidth: '14rem' }} />
      <div className="flex-1">
        {mainContent}
      </div>
    </div>
  );
}

type AdvancedDocumentSearchFormValues = {
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

export function AdvancedDocumentSearch() {
  const navigate = useNavigate();
  const location = useLocation();
  const existingParams = useMemo(() => parseDocumentListHash(location.hash), [location.hash]);
  const { data: documentTypes, isPending: isDocumentTypesPending, isFetching: isDocumentTypesFetching } = useDocumentTypes({ page: 1, per_page: 200, sf: 'name' });
  const { data: metadataTypes, isPending: isMetadataTypesPending, isFetching: isMetadataTypesFetching } = useMetadataTypes({ page: 1, per_page: 200, sf: 'name' });
  const { data: cabinets, isPending: isCabinetsPending, isFetching: isCabinetsFetching } = useCabinets({ page: 1, per_page: MAX_CABINETS, sf: 'name' });
  const { data: tags, isPending: isTagsPending, isFetching: isTagsFetching } = useTags({ page: 1, per_page: 200, sf: 'name' });
  const { control, handleSubmit, reset, formState: { isSubmitting } } = useForm<AdvancedDocumentSearchFormValues>({
    defaultValues: {
      match_any: !!existingParams.match_any,
      q: parseBasicDocumentSearchHash(location.hash) ? '' : existingParams.q ?? '',
      text: parseBasicDocumentSearchHash(location.hash) ? '' : existingParams.text ?? '',
      document_type_id: existingParams.document_type_id ?? null,
      metadata_value: parseBasicDocumentSearchHash(location.hash) ? '' : existingParams.metadata_value ?? '',
      metadata_type_id: existingParams.metadata_type_id ?? null,
      filename: existingParams.filename ?? '',
      file_content_type: existingParams.file_content_type ?? '',
      cabinet_id: existingParams.cabinet_id ?? null,
      tag_id: existingParams.tag_id ?? null,
      duplicates: !!existingParams.duplicates,
      duplicate_checksum: !!existingParams.duplicate_checksum,
    },
  });

  const onSubmit = (values: AdvancedDocumentSearchFormValues) => {
    const nextParams: DocumentListParams = {
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
      duplicates: values.duplicates || undefined,
      duplicate_checksum: values.duplicate_checksum || undefined,
    };

    navigate({
      pathname: '/documents',
      hash: serializeDocumentListHash(nextParams),
    });
  };

  const resetForm = () => {
    reset({
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
    });
  };

  return (
    <Card title="Advanced Document Search">
      <form onSubmit={handleSubmit(onSubmit)}>
        <div className="grid p-fluid">
          <div className="col-12">
            <Controller
              name="match_any"
              control={control}
              render={({ field }) => (
                <div className="flex align-items-center gap-2">
                  <Checkbox
                    inputId="advanced-search-match-any"
                    checked={field.value}
                    onChange={(event) => field.onChange(!!event.checked)}
                  />
                  <label htmlFor="advanced-search-match-any">Match Any</label>
                </div>
              )}
            />
            <small className="text-color-secondary block mt-2">When checked, documents can match any search criterion instead of all criteria.</small>
          </div>

          <div className="col-12 md:col-6">
            <label htmlFor="advanced-search-title" className="font-medium mb-2 block">Title</label>
            <Controller
              name="q"
              control={control}
              render={({ field }) => (
                <InputText
                  id="advanced-search-title"
                  value={field.value}
                  onChange={field.onChange}
                  placeholder="Title contains..."
                />
              )}
            />
          </div>

          <div className="col-12 md:col-6">
            <label htmlFor="advanced-search-text" className="font-medium mb-2 block">Document Text / OCR Search</label>
            <Controller
              name="text"
              control={control}
              render={({ field }) => (
                <InputText
                  id="advanced-search-text"
                  value={field.value}
                  onChange={field.onChange}
                  placeholder="Document text or OCR search..."
                />
              )}
            />
          </div>

          <div className="col-12 md:col-6">
            <label htmlFor="advanced-search-metadata-value" className="font-medium mb-2 block">Metadata Value</label>
            <Controller
              name="metadata_value"
              control={control}
              render={({ field }) => (
                <InputText
                  id="advanced-search-metadata-value"
                  value={field.value}
                  onChange={field.onChange}
                  placeholder="Metadata value contains..."
                />
              )}
            />
          </div>

          <div className="col-12 md:col-6">
            <label htmlFor="advanced-search-metadata-type" className="font-medium mb-2 block">Metadata Type</label>
            <Controller
              name="metadata_type_id"
              control={control}
              render={({ field }) => (
                <Dropdown
                  id="advanced-search-metadata-type"
                  value={field.value}
                  onChange={(event) => field.onChange(event.value ?? null)}
                  optionLabel="name"
                  optionValue="id"
                  options={metadataTypes?.items ?? []}
                  loading={isMetadataTypesPending || isMetadataTypesFetching}
                  placeholder="Any metadata type"
                  showClear
                />
              )}
            />
          </div>

          <div className="col-12 md:col-6">
            <label htmlFor="advanced-search-filename" className="font-medium mb-2 block">Filename</label>
            <Controller
              name="filename"
              control={control}
              render={({ field }) => (
                <InputText
                  id="advanced-search-filename"
                  value={field.value}
                  onChange={field.onChange}
                  placeholder="Filename contains..."
                />
              )}
            />
          </div>

          <div className="col-12 md:col-6">
            <label htmlFor="advanced-search-file-content-type" className="font-medium mb-2 block">File Content Type</label>
            <Controller
              name="file_content_type"
              control={control}
              render={({ field }) => (
                <InputText
                  id="advanced-search-file-content-type"
                  value={field.value}
                  onChange={field.onChange}
                  placeholder="Content type contains..."
                />
              )}
            />
          </div>

          <div className="col-12 md:col-6">
            <label htmlFor="advanced-search-cabinet" className="font-medium mb-2 block">Cabinet</label>
            <Controller
              name="cabinet_id"
              control={control}
              render={({ field }) => (
                <Dropdown
                  id="advanced-search-cabinet"
                  value={field.value}
                  onChange={(event) => field.onChange(event.value ?? null)}
                  optionLabel="displayName"
                  optionValue="id"
                  options={cabinets?.items ?? []}
                  loading={isCabinetsPending || isCabinetsFetching}
                  placeholder="Any cabinet"
                  showClear
                />
              )}
            />
          </div>

          <div className="col-12 md:col-6">
            <label htmlFor="advanced-search-tag" className="font-medium mb-2 block">Tag</label>
            <Controller
              name="tag_id"
              control={control}
              render={({ field }) => (
                <Dropdown
                  id="advanced-search-tag"
                  value={field.value}
                  onChange={(event) => field.onChange(event.value ?? null)}
                  optionLabel="name"
                  optionValue="id"
                  options={tags?.items ?? []}
                  loading={isTagsPending || isTagsFetching}
                  placeholder="Any tag"
                  showClear
                />
              )}
            />
          </div>

          <div className="col-12 md:col-6">
            <label htmlFor="advanced-search-document-type" className="font-medium mb-2 block">Document Type</label>
            <Controller
              name="document_type_id"
              control={control}
              render={({ field }) => (
                <Dropdown
                  id="advanced-search-document-type"
                  value={field.value}
                  onChange={(event) => field.onChange(event.value ?? null)}
                  optionLabel="name"
                  optionValue="id"
                  options={documentTypes?.items ?? []}
                  loading={isDocumentTypesPending || isDocumentTypesFetching}
                  placeholder="Any document type"
                  showClear
                />
              )}
            />
          </div>

          <div className="col-12 md:col-6 flex align-items-end">
            <Controller
              name="duplicates"
              control={control}
              render={({ field }) => (
                <div className="flex align-items-center gap-2 mb-2">
                  <Checkbox
                    inputId="advanced-search-duplicates"
                    checked={field.value}
                    onChange={(event) => field.onChange(!!event.checked)}
                  />
                  <label htmlFor="advanced-search-duplicates">Find documents with duplicate title</label>
                </div>
              )}
            />
          </div>

          <div className="col-12 md:col-6 flex align-items-end">
            <Controller
              name="duplicate_checksum"
              control={control}
              render={({ field }) => (
                <div className="flex align-items-center gap-2 mb-2">
                  <Checkbox
                    inputId="advanced-search-duplicate-checksum"
                    checked={field.value}
                    onChange={(event) => field.onChange(!!event.checked)}
                  />
                  <label htmlFor="advanced-search-duplicate-checksum">Find documents with duplicate file checksum</label>
                </div>
              )}
            />
          </div>
        </div>

        <div className="flex flex-column sm:flex-row justify-content-end gap-2 mt-4">
          <Button label="Search" type="submit" icon="pi pi-search" raised disabled={isSubmitting} />
          <Button label="Reset" type="button" icon="pi pi-refresh" severity="secondary" outlined onClick={resetForm} />
          <Button label="Cancel" type="button" icon="pi pi-times" severity="secondary" text onClick={() => navigate('/documents')} />
        </div>
      </form>
    </Card>
  );
}

type DocumentPropertiesFormValues = {
  title: string;
  document_type_id: number | null;
};

export function EditDocumentProperties() {
  const navigate = useNavigate();
  const id = useId('id');
  const saveDocument = useSaveDocument();
  const { isLoading, isError, data: doc, error } = useDocument(id);
  const { data: documentTypes, isPending: isDocumentTypesPending, isFetching: isDocumentTypesFetching } = useDocumentTypes({ page: 1, per_page: 200, sf: 'name' });
  const { data: cabinetOptions } = useCabinets({ page: 1, per_page: MAX_CABINETS });
  const { data: tagOptions } = useTags({ page: 1, per_page: 200 });
  const cabinetLookup = useMemo(() => {
    const lookup: Record<number, Cabinet> = {};
    for (const cabinet of cabinetOptions?.items ?? []) {
      lookup[cabinet.id] = cabinet;
    }
    return lookup;
  }, [cabinetOptions?.items]);
  const tagLookup = useMemo(() => {
    const lookup: Record<number, TagModel> = {};
    for (const tag of tagOptions?.items ?? []) {
      lookup[tag.id] = tag;
    }
    return lookup;
  }, [tagOptions?.items]);
  const cabinetItems = (doc?.cabinet_ids ?? [])
    .map((cabinetId) => cabinetLookup[cabinetId])
    .filter((cabinet): cabinet is Cabinet => !!cabinet);
  const tagItems = (doc?.tag_ids ?? [])
    .map((tagId) => tagLookup[tagId])
    .filter((tag): tag is TagModel => !!tag);
  const {
    control,
    handleSubmit,
    formState: { errors, isSubmitting },
  } = useForm<DocumentPropertiesFormValues>({
    mode: 'onChange',
    values: {
      title: doc?.title ?? '',
      document_type_id: doc?.document_type_id ?? null,
    },
  });

  const submitter = async (values: DocumentPropertiesFormValues) => {
    await saveDocument.mutateAsync({
      id,
      title: values.title,
      document_type_id: values.document_type_id ?? undefined,
    });
    navigate(`/documents/${id}/preview`);
  };

  if (isError)
    return <Message severity="error" text={error.message} />;
  if (isLoading)
    return <div>Loading</div>;

  return (
    <DocumentViewLayout documentId={id}>
      <Card title={`Document Properties: ${doc?.title ?? ''}`}>
        <form onSubmit={handleSubmit(submitter)}>
          <div className="grid p-fluid">
            <div className="col-12 md:col-6">
              <label htmlFor="document_title" className="font-medium mb-2 block">Title</label>
              <Controller
                name="title"
                control={control}
                rules={{
                  required: 'Title is required',
                  minLength: { value: 2, message: 'Title must be at least 2 characters' },
                }}
                render={({ field }) => (
                  <InputText
                    id="document_title"
                    {...field}
                    className={classNames({ 'p-invalid': !!errors.title })}
                    placeholder="Enter document title"
                  />
                )}
              />
              {errors.title?.message && <small className="p-error">{String(errors.title.message)}</small>}
            </div>

            <div className="col-12 md:col-6">
              <label htmlFor="document_type_id" className="font-medium mb-2 block">Document Type</label>
              <Controller
                name="document_type_id"
                control={control}
                rules={{ required: 'Document type is required' }}
                render={({ field }) => (
                  <Dropdown
                    id="document_type_id"
                    value={field.value}
                    onChange={(event) => field.onChange(event.value as number)}
                    options={documentTypes?.items ?? []}
                    optionLabel="name"
                    optionValue="id"
                    placeholder="Select a document type"
                    loading={isDocumentTypesPending || isDocumentTypesFetching}
                    className={classNames('w-full', { 'p-invalid': !!errors.document_type_id })}
                  />
                )}
              />
              {errors.document_type_id?.message && <small className="p-error">{String(errors.document_type_id.message)}</small>}
            </div>

            <div className="col-12">
              <label className="font-medium mb-2 block">Tags</label>
              {tagItems.length > 0 ? (
                <ul className="aut-document-tags">
                  {tagItems.map((tag) => (
                    <li key={tag.id}>
                      <Link to={`/tags/${tag.id}/documents`}>
                        <Badge value={tag.name} className="aut-document-tag" style={{ backgroundColor: `#${tag.color}` }} />
                      </Link>
                    </li>
                  ))}
                </ul>
              ) : (
                <span>No tags</span>
              )}
            </div>

            <div className="col-12">
              <label className="font-medium mb-2 block">Cabinets</label>
              {cabinetItems.length > 0 ? (
                <ul className="aut-document-cabinets">
                  {cabinetItems.map((cabinet) => (
                    <li key={cabinet.id}>
                      <Link to={`/cabinets/${cabinet.id}/documents`}>
                        <Badge value={`🗄️ ${cabinet.displayName ?? cabinet.name ?? cabinet.slug}`} severity="secondary" />
                      </Link>
                    </li>
                  ))}
                </ul>
              ) : (
                <span>No cabinets</span>
              )}
            </div>
          </div>

          <div className="text-end">
            {saveDocument.isError && (
              <Message className="float-start" severity="error" text={saveDocument.error.message} />
            )}

            <Button label="Save" type="submit" icon="pi pi-check" raised disabled={saveDocument.isPending || isSubmitting} />
            <Button label="Cancel" type="button" severity="secondary" icon="pi pi-times" raised onClick={() => navigate(`/documents/${id}/preview`)} />
          </div>
        </form>
      </Card>
    </DocumentViewLayout>
  );
}

export default function UploadDocument() {
    const toast = useRef<Toast>(null);
    const [totalSize, setTotalSize] = useState(0);
    const fileUploadRef = useRef<FileUpload>(null);
    const [title, setTitle] = useState('');
    const [documentTypeId, setDocumentTypeId] = useState<number | null>(null);
    const [isUploading, setIsUploading] = useState(false);
    const [uploadProgress, setUploadProgress] = useState<number | null>(null);
    const [uploadStatus, setUploadStatus] = useState('');
    const { data: documentTypes, isPending: isDocumentTypesPending, isFetching: isDocumentTypesFetching } = useDocumentTypes({ page: 1, per_page: 200, sf: 'name' });
    const defaultDocumentTypeId = documentTypes?.items?.find((item) => item.name === 'Unspecified' || item.slug === 'unspecified')?.id ?? null;
    const effectiveDocumentTypeId = documentTypeId ?? defaultDocumentTypeId;
    
    const onTemplateSelect = (e:FileUploadSelectEvent) => {
        let _totalSize = totalSize;
        const files = e.files;

        files.forEach((_value: FileUploadFile, key: number) => {
            _totalSize += files[key].size || 0;
        });

        setTotalSize(_totalSize);
    };

    const onTemplateUpload = (e:FileUploadUploadEvent) => {
        let _totalSize = 0;

        e.files.forEach((file) => {
            _totalSize += file.size || 0;
        });

        setTotalSize(_totalSize);
        toast.current?.show({ severity: 'info', summary: 'Success', detail: 'File Uploaded' });
    };

    const onTemplateRemove = (file: File, callback: (event: React.SyntheticEvent) => void, event: React.SyntheticEvent) => {
        setTotalSize(totalSize - file.size);
        callback(event);
    };

    const onTemplateClear = () => {
        setTotalSize(0);
    };

    const headerTemplate = (options: { className: string; chooseButton: ReactNode; uploadButton: ReactNode; cancelButton: ReactNode }) => {
        const { className, chooseButton, uploadButton, cancelButton } = options;
        const formatedValue = fileUploadRef?.current ? fileUploadRef.current.formatSize(totalSize) : '0 B';

        return (
            <div className={className} style={{ backgroundColor: 'transparent', display: 'flex', alignItems: 'center' }}>
                {chooseButton}
                {uploadButton}
                {cancelButton}
                <div className="flex align-items-center gap-3 ml-auto">
                    <span>{formatedValue} selected</span>
                </div>
            </div>
        );
    };

    const itemTemplate = (file: FileUploadFile, props: ItemTemplateOptions) => {
        return (
            <div className="flex align-items-center flex-wrap">
                <div className="flex align-items-center" style={{ width: '40%' }}>
                    <img alt={file.name} role="presentation" src={file.objectURL} width={100} />
                    <span className="flex flex-column text-left ml-3">
                        {file.name}
                        <small>{new Date().toLocaleDateString()}</small>
                    </span>
                </div>
                <Tag value={props.formatSize} severity="warning" className="px-3 py-2" />
                <Button
                    type="button" icon="pi pi-times" className="p-button-outlined p-button-rounded p-button-danger ml-auto"
                    onClick={(event) => onTemplateRemove(file, props.onRemove, event)}
                  />
            </div>
        );
    };

    const uploadFile = (
        formData: FormData,
        file: File,
        onProgress: (loaded: number) => void,
        onUploadComplete: () => void,
    ) => {
        return new Promise<void>((resolve, reject) => {
            const xhr = new XMLHttpRequest();
            xhr.open('POST', apiUrl('api/v1/documents'));
            xhr.withCredentials = true;
            const token = getAccessToken();
            if (token) xhr.setRequestHeader('Authorization', `Bearer ${token}`);

            xhr.upload.onprogress = (event) => {
                if (event.lengthComputable) {
                    onProgress(event.loaded);
                }
            };
            xhr.upload.onload = onUploadComplete;

            xhr.onload = () => {
                if (xhr.status >= 200 && xhr.status < 300) {
                    resolve();
                    return;
                }

                let detail = `Upload failed (${xhr.status})`;
                try {
                    const data = JSON.parse(xhr.responseText);
                    if (typeof data?.message === 'string') detail = data.message;
                } catch {
                    if (xhr.responseText) detail = xhr.responseText;
                }
                reject(new Error(detail));
            };

            xhr.onerror = () => reject(new Error(`Upload failed for ${file.name}`));
            xhr.onabort = () => reject(new Error(`Upload cancelled for ${file.name}`));
            xhr.send(formData);
        });
    };

    const uploadHandler = async (event: FileUploadHandlerEvent) => {
        if (!effectiveDocumentTypeId) {
            toast.current?.show({ severity: 'warn', summary: 'Missing type', detail: 'Select a document type before uploading.' });
            return;
        }

        const files = event.files ?? [];
        if (!files.length) return;

        setIsUploading(true);
        setUploadProgress(0);
        setUploadStatus(files.length === 1 ? `Uploading ${files[0].name}...` : `Uploading 1 of ${files.length}: ${files[0].name}`);

        const uploadedBytesByFile = new Map<File, number>();
        const totalBytes = files.reduce((sum, file) => sum + (file.size || 0), 0);

        const updateProgress = (file: File, loaded: number) => {
            uploadedBytesByFile.set(file, loaded);
            const loadedBytes = Array.from(uploadedBytesByFile.values()).reduce((sum, value) => sum + value, 0);
            setUploadProgress(totalBytes > 0 ? Math.min(100, Math.round((loadedBytes / totalBytes) * 100)) : null);
        };

        const uploadOne = async (file: File) => {
            const formData = new FormData();
            const trimmedTitle = title.trim();
            const resolvedTitle = trimmedTitle || file.name;
            formData.append('title', resolvedTitle);
            formData.append('document_type_id', String(effectiveDocumentTypeId));
            formData.append('file', file);

            await uploadFile(
                formData,
                file,
                (loaded) => updateProgress(file, loaded),
                () => {
                    updateProgress(file, file.size || 0);
                    setUploadStatus(`Processing ${file.name}...`);
                },
            );
            uploadedBytesByFile.set(file, file.size || 0);
        };

        const failures: unknown[] = [];
        for (const [index, file] of files.entries()) {
            setUploadStatus(files.length === 1 ? `Uploading ${file.name}...` : `Uploading ${index + 1} of ${files.length}: ${file.name}`);
            try {
                await uploadOne(file);
            } catch (error) {
                failures.push(error);
            }
        }

        if (failures.length) {
            const message = failures[0] instanceof Error
                ? failures[0].message
                : 'Some files failed to upload.';
            toast.current?.show({
                severity: 'error',
                summary: 'Upload incomplete',
                detail: message,
            });
            setIsUploading(false);
            setUploadProgress(null);
            setUploadStatus('');
            return;
        }

        event.options.clear();
        setTotalSize(0);
        setTitle('');
        setDocumentTypeId(null);
        setIsUploading(false);
        setUploadProgress(null);
        setUploadStatus('');
        const label = files.length === 1 ? 'File uploaded.' : `${files.length} files uploaded.`;
        toast.current?.show({ severity: 'success', summary: 'Success', detail: label });
    };

    const emptyTemplate = () => {
        return (
            <div className="flex align-items-center flex-column">
                <i className="pi pi-image mt-3 p-5" style={{ fontSize: '5em', borderRadius: '50%', backgroundColor: 'var(--surface-b)', color: 'var(--surface-d)' }}></i>
                <span style={{ fontSize: '1.2em', color: 'var(--text-color-secondary)' }} className="my-5">
                    Drag and Drop Image Here
                </span>
            </div>
        );
    };

    const chooseOptions = { label: 'Choose Files', icon: 'pi pi-fw pi-images', className: 'custom-choose-btn p-button-rounded p-button-outlined' };
    const uploadOptions = { label: 'Upload', icon: 'pi pi-fw pi-cloud-upload', className: 'custom-upload-btn p-button-success p-button-rounded p-button-outlined' };
    const cancelOptions = { label: 'Clear', icon: 'pi pi-fw pi-times', className: 'custom-cancel-btn p-button-danger p-button-rounded p-button-outlined' };

    return (
        <div>
            <Toast ref={toast}></Toast>

            <div className="grid p-fluid mb-4">
                <div className="col-12 md:col-6">
                    <label htmlFor="document_title" className="font-medium mb-2 block">Title</label>
                    <InputText
                        id="document_title"
                        value={title}
                        onChange={(event) => setTitle(event.target.value)}
                        placeholder="Enter document title"
                        className="w-full"
                        disabled={isUploading}
                    />
                </div>
                <div className="col-12 md:col-6">
                    <label htmlFor="document_type_id" className="font-medium mb-2 block">Document Type</label>
                    <Dropdown
                        id="document_type_id"
                        value={effectiveDocumentTypeId}
                        onChange={(event) => setDocumentTypeId(event.value as number)}
                        optionLabel="name"
                        optionValue="id"
                        placeholder="Select a document type"
                        options={documentTypes?.items ?? []}
                        loading={isDocumentTypesPending || isDocumentTypesFetching}
                        className="w-full"
                        disabled={isUploading}
                    />
                </div>
            </div>

            <Tooltip target=".custom-choose-btn" content="Choose" position="bottom" />
            <Tooltip target=".custom-upload-btn" content="Upload" position="bottom" />
            <Tooltip target=".custom-cancel-btn" content="Clear" position="bottom" />

            <FileUpload ref={fileUploadRef} name="file" customUpload uploadHandler={uploadHandler} multiple
                onUpload={onTemplateUpload} onSelect={onTemplateSelect} onError={onTemplateClear} onClear={onTemplateClear}
                headerTemplate={headerTemplate} emptyTemplate={emptyTemplate}
                itemTemplate={itemTemplate as (file: object, options: ItemTemplateOptions) => React.ReactNode}
                chooseOptions={chooseOptions} uploadOptions={uploadOptions} cancelOptions={cancelOptions}
                disabled={isUploading} />

            {isUploading && (
                <div className="aut-upload-progress-panel mt-4" role="status" aria-live="polite">
                    <div className="aut-upload-progress-status">
                        <span className="pi pi-spin pi-spinner" aria-hidden="true" />
                        <span>{uploadStatus}</span>
                        {uploadProgress !== null && <span className="aut-upload-progress-percent">{uploadProgress}%</span>}
                    </div>
                    {uploadProgress === null ? (
                        <ProgressBar mode="indeterminate" showValue={false} className="aut-upload-progress-bar" />
                    ) : (
                        <ProgressBar value={uploadProgress} showValue={false} className="aut-upload-progress-bar" />
                    )}
                </div>
            )}
        </div>
    )
}
