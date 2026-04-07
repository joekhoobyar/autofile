import { useMemo, useRef, useState, type ReactNode } from 'react';
import { Link, useNavigate, useParams } from 'react-router-dom';
import { Controller, useForm } from 'react-hook-form';

import { Card } from 'primereact/card';
import { Checkbox } from 'primereact/checkbox';
import { DataView, DataViewLayoutOptions, type DataViewPageEvent } from 'primereact/dataview';
import { Dropdown } from 'primereact/dropdown';
import { Dialog } from 'primereact/dialog';
import { classNames } from 'primereact/utils';
import { format } from "date-fns";

import { apiFetchRaw } from '../api';
import { useDocuments, useDocumentThumbnail } from '../queries/useDocuments';
import { useDocumentIndex } from '../queries/useDocumentIndexes';
import { useDocumentIndexValueAncestors } from '../queries/useDocumentIndexValues';
import { type Document, type DocumentListParams } from '../models/document';
import { useMetadataTypesMap } from '../queries/useMetadataTypes';
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
      .filter((cabinet): cabinet is Cabinet => !!cabinet);
    const tagItems = (doc.tag_ids ?? [])
      .map((id) => tagLookup[id])
      .filter((tag): tag is TagModel => !!tag);

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
      .filter((cabinet): cabinet is Cabinet => !!cabinet);
    const tagItems = (doc.tag_ids ?? [])
      .map((id) => tagLookup[id])
      .filter((tag): tag is TagModel => !!tag);

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
  const params = useParams();
  const initialListParams: DocumentListParams = {
    per_page: 12,
    page: 1,
    sf: 'created_at',
    sd: true,
  };
  const [listParams, setListParams] = useState<DocumentListParams>(initialListParams);
  const tagId = params.tagId ? Number.parseInt(params.tagId) : undefined;
  const cabinetId = params.cabinetId ? Number.parseInt(params.cabinetId) : undefined;
  const documentIndexValueId = params.documentIndexValueId ? Number.parseInt(params.documentIndexValueId) : undefined;
  const documentIndexId = params.documentIndexId ? Number.parseInt(params.documentIndexId) : undefined;
  const effectiveListParams: DocumentListParams = {
    ...listParams,
    tag_id: tagId,
    cabinet_id: cabinetId,
    document_index_value_id: documentIndexValueId,
  };
  const showIndexMenu = !!documentIndexId && !!documentIndexValueId;
  const { data: indexAncestors } = useDocumentIndexValueAncestors(documentIndexId ?? 0, documentIndexValueId ?? 0);
  const { data: documentIndex } = useDocumentIndex(documentIndexId ?? 0, { enabled: !!documentIndexId });
  const [layout, setLayout] = useState<'list' | 'grid'>('grid');
  const [previewVisible, setPreviewVisible] = useState(false);
  const [previewSrc, setPreviewSrc] = useState<string | undefined>(undefined);
  const [previewTitle, setPreviewTitle] = useState('');
  const [selectedIds, setSelectedIds] = useState<Set<number>>(new Set());
  const { isPending, data, isFetching } = useDocuments(effectiveListParams);
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

  const activeFilterChips = useMemo(() => {
    const chips: Array<{ key: string; label: string }> = [];
    if (tagId && tagLookup[tagId]) {
      chips.push({ key: `tag-${tagId}`, label: `🏷️ ${tagLookup[tagId].name}` });
    }
    if (cabinetId && cabinetLookup[cabinetId]) {
      const cabinet = cabinetLookup[cabinetId];
      chips.push({ key: `cabinet-${cabinetId}`, label: `🗄️ ${cabinet.displayName ?? cabinet.name ?? cabinet.slug}` });
    }
    return chips;
  }, [tagId, cabinetId, tagLookup, cabinetLookup]);

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

  const onPage = (event: DataViewPageEvent) => {
    setListParams({ ...listParams, page: event.page + 1, per_page: event.rows });
  };

  const onSortChange = (value: string | undefined) => {
    if (!value) {
      setListParams({ ...listParams, sf: undefined, sd: undefined, page: 1 });
      return;
    }

    const [field, direction] = value.split(':');
    setListParams({
      ...listParams,
      sf: field,
      sd: direction === 'desc',
      page: 1,
    });
  };

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
    return (
      <div className="flex justify-content-end">
        <DataViewLayoutOptions layout={layout} onChange={(e) => setLayout(e.value as 'list' | 'grid')} />
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
              navigate('/documents');
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
        const value = totalSize / 10000;
        const formatedValue = fileUploadRef?.current ? fileUploadRef.current.formatSize(totalSize) : '0 B';

        return (
            <div className={className} style={{ backgroundColor: 'transparent', display: 'flex', alignItems: 'center' }}>
                {chooseButton}
                {uploadButton}
                {cancelButton}
                <div className="flex align-items-center gap-3 ml-auto">
                    <span>{formatedValue} / 1 MB</span>
                    <ProgressBar value={value} showValue={false} style={{ width: '10rem', height: '12px' }}></ProgressBar>
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

    const uploadHandler = async (event: FileUploadHandlerEvent) => {
        if (!effectiveDocumentTypeId) {
            toast.current?.show({ severity: 'warn', summary: 'Missing type', detail: 'Select a document type before uploading.' });
            return;
        }

        const files = event.files ?? [];
        if (!files.length) return;

        const uploadOne = async (file: File) => {
            const formData = new FormData();
            const trimmedTitle = title.trim();
            const resolvedTitle = trimmedTitle || file.name;
            formData.append('title', resolvedTitle);
            formData.append('document_type_id', String(effectiveDocumentTypeId));
            formData.append('file', file);

            const res = await apiFetchRaw('api/v1/documents', {
                method: 'POST',
                body: formData,
            });

            if (!res.ok) {
                let detail = `Upload failed (${res.status})`;
                try {
                    const data = await res.json();
                    if (typeof data?.message === 'string') {
                        detail = data.message;
                    }
                } catch {
                    try {
                        const text = await res.text();
                        if (text) detail = text;
                    } catch {
                        // ignore
                    }
                }
                throw new Error(detail);
            }
        };

        const results = await Promise.allSettled(files.map((file) => uploadOne(file)));
        const failures = results.filter((result) => result.status === 'rejected');

        if (failures.length) {
            const message = failures[0].status === 'rejected' && failures[0].reason instanceof Error
                ? failures[0].reason.message
                : 'Some files failed to upload.';
            toast.current?.show({
                severity: 'error',
                summary: 'Upload incomplete',
                detail: message,
            });
            return;
        }

        event.options.clear();
        setTotalSize(0);
        setTitle('');
        setDocumentTypeId(null);
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
                chooseOptions={chooseOptions} uploadOptions={uploadOptions} cancelOptions={cancelOptions} />
        </div>
    )
}
