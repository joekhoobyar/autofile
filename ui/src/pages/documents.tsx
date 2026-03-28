import { useMemo, useRef, useState, type ReactNode } from 'react';
import { Link, useParams } from 'react-router-dom';

import { Card } from 'primereact/card';
import { Checkbox } from 'primereact/checkbox';
import { DataView, DataViewLayoutOptions, type DataViewPageEvent } from 'primereact/dataview';
import { Dropdown } from 'primereact/dropdown';
import { Dialog } from 'primereact/dialog';
import { classNames } from 'primereact/utils';
import { format } from "date-fns";
import { confirmDialog, ConfirmDialog } from 'primereact/confirmdialog';

import { apiFetchRaw } from '../api';
import { useDeleteDocument, useDocuments, useDocumentThumbnail, useRemoveCabinetDocument, useRemoveTagDocument, useSaveCabinetDocument, useSaveTagDocument } from '../queries/useDocuments';
import { type Document, type DocumentListParams } from '../models/document';
import { useMetadataTypesMap } from '../queries/useMetadataTypes';
import { useDocumentTypes, useDocumentTypesMap } from '../queries/useDocumentTypes';
import { Menu } from 'primereact/menu';
import { Button } from 'primereact/button';
import type { MenuItem } from 'primereact/menuitem';
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
      if (src.startsWith('blob:')) {
        URL.revokeObjectURL(src);
      }
    }
  };

  const handleError = () => {
    if (src) {
      setErrorSrc(src);
      if (src.startsWith('blob:')) {
        URL.revokeObjectURL(src);
      }
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
            style={{ maxHeight: '200px', display: isLoaded ? 'block' : 'none' }}
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
      <div className="col-12 aut-document-list" key={doc.id}>
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
              <header className="flex align-items-center gap-2">
                <Checkbox
                  inputId={`document-select-${doc.id}`}
                  onChange={(event) => onSelectionChange(doc.id, !!event.checked)}
                  checked={selected}
                />
                <Link to={`${doc.id}/metadata`}>{doc.title}</Link>
              </header>
            </div>
            <aside className="flex flex-column align-items-center sm:align-items-start">
              <ul className="aut-document-metadata">
                <li><span>Type</span>: {ddt?.[doc.document_type_id]?.name}</li>
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

    return (
      <div className="col-12 sm:col-6 lg:col-4 xl:col-2 p-2 aut-document-grid" key={doc.id}>
        <div className="border-1 surface-border surface-card border-round">
          <section className="flex flex-column aut-document">
            <header className="flex align-items-center gap-2 aut-document-header">
              <Checkbox
                inputId={`document-select-${doc.id}`}
                onChange={(event) => onSelectionChange(doc.id, !!event.checked)}
                checked={selected}
              />
              <Link to={`${doc.id}/metadata`}>{doc.title}</Link>
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
  const params = useParams();
  const initialListParams: DocumentListParams = {
    page: 1,
    sf: 'created_at',
    sd: true,
  };
  const [listParams, setListParams] = useState<DocumentListParams>(initialListParams);
  const tagId = params.tagId ? Number.parseInt(params.tagId) : undefined;
  const cabinetId = params.cabinetId ? Number.parseInt(params.cabinetId) : undefined;
  const effectiveListParams: DocumentListParams = {
    ...listParams,
    tag_id: tagId,
    cabinet_id: cabinetId,
  };
  const [layout, setLayout] = useState<'list' | 'grid'>('grid');
  const [previewVisible, setPreviewVisible] = useState(false);
  const [previewSrc, setPreviewSrc] = useState<string | undefined>(undefined);
  const [previewTitle, setPreviewTitle] = useState('');
  const [selectedIds, setSelectedIds] = useState<Set<number>>(new Set());
  const [addToCabinetVisible, setAddToCabinetVisible] = useState(false);
  const [selectedCabinetId, setSelectedCabinetId] = useState<number | null>(null);
  const [removeFromCabinetVisible, setRemoveFromCabinetVisible] = useState(false);
  const [removeCabinetId, setRemoveCabinetId] = useState<number | null>(null);
  const [addTagVisible, setAddTagVisible] = useState(false);
  const [selectedTagId, setSelectedTagId] = useState<number | null>(null);
  const [removeTagVisible, setRemoveTagVisible] = useState(false);
  const [removeTagId, setRemoveTagId] = useState<number | null>(null);
  const { isPending, data, isFetching } = useDocuments(effectiveListParams);
  const deleteDocument = useDeleteDocument();
  const saveCabinetDocument = useSaveCabinetDocument();
  const removeCabinetDocument = useRemoveCabinetDocument();
  const saveTagDocument = useSaveTagDocument();
  const removeTagDocument = useRemoveTagDocument();
  const { data: cabinetOptions, isPending: isCabinetsPending, isFetching: isCabinetsFetching } = useCabinets({ page: 1, per_page: MAX_CABINETS });
  const { data: tagOptions, isPending: isTagsPending, isFetching: isTagsFetching } = useTags({ page: 1, per_page: 200 });
  const actionMenu = useRef<Menu>(null);
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
    setListParams({ ...listParams, page: event.page, per_page: event.rows });
  };

  const onSortChange = (value: string | undefined) => {
    if (!value) {
      setListParams({ ...listParams, sf: undefined, sd: undefined, page: 0 });
      return;
    }

    const [field, direction] = value.split(':');
    setListParams({
      ...listParams,
      sf: field,
      sd: direction === 'desc',
      page: 0,
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

  const deleteSelectedDocuments = async () => {
    if (!selectedIds.size) return;
    const ids = Array.from(selectedIds);
    await Promise.all(ids.map((id) => deleteDocument.mutateAsync(id)));
    setSelectedIds(new Set());
  };

  const confirmDeleteSelectedDocuments = () => {
    if (!selectedIds.size) return;
    const count = selectedIds.size;
    const label = count === 1 ? 'document' : 'documents';
    confirmDialog({
      message: `Are you sure you want to delete ${count} ${label}?`,
      header: 'Delete Documents',
      icon: 'pi pi-trash',
      defaultFocus: 'reject',
      acceptClassName: 'p-button-danger',
      accept: () => void deleteSelectedDocuments(),
    });
  };

  const openAddToCabinetDialog = () => {
    if (!selectedIds.size) return;
    setAddToCabinetVisible(true);
  };

  const closeAddToCabinetDialog = () => {
    setAddToCabinetVisible(false);
    setSelectedCabinetId(null);
  };

  const saveAddToCabinet = async () => {
    if (!selectedCabinetId || !selectedIds.size) return;
    const documents = Array.from(selectedIds).map((id) => ({ document_id: id }));
    await saveCabinetDocument.mutateAsync({ cabinet_id: selectedCabinetId, documents });
    closeAddToCabinetDialog();
    setSelectedIds(new Set());
  };

  const openRemoveFromCabinetDialog = () => {
    if (!selectedIds.size) return;
    setRemoveFromCabinetVisible(true);
  };

  const closeRemoveFromCabinetDialog = () => {
    setRemoveFromCabinetVisible(false);
    setRemoveCabinetId(null);
  };

  const saveRemoveFromCabinet = async () => {
    if (!removeCabinetId || !selectedIds.size) return;
    const documents = Array.from(selectedIds);
    await removeCabinetDocument.mutateAsync({ cabinet_id: removeCabinetId, documents });
    closeRemoveFromCabinetDialog();
    setSelectedIds(new Set());
  };

  const openAddTagDialog = () => {
    if (!selectedIds.size) return;
    setAddTagVisible(true);
  };

  const closeAddTagDialog = () => {
    setAddTagVisible(false);
    setSelectedTagId(null);
  };

  const saveAddTag = async () => {
    if (!selectedTagId || !selectedIds.size) return;
    const documents = Array.from(selectedIds).map((id) => ({ document_id: id }));
    await saveTagDocument.mutateAsync({ tag_id: selectedTagId, documents });
    closeAddTagDialog();
    setSelectedIds(new Set());
  };

  const openRemoveTagDialog = () => {
    if (!selectedIds.size) return;
    setRemoveTagVisible(true);
  };

  const closeRemoveTagDialog = () => {
    setRemoveTagVisible(false);
    setRemoveTagId(null);
  };

  const saveRemoveTag = async () => {
    if (!removeTagId || !selectedIds.size) return;
    const documents = Array.from(selectedIds);
    await removeTagDocument.mutateAsync({ tag_id: removeTagId, documents });
    closeRemoveTagDialog();
    setSelectedIds(new Set());
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
      <div className="flex align-items-center gap-3">
        <Dropdown
          value={sortValue}
          options={sortOptions}
          placeholder="Sort by"
          onChange={(event) => onSortChange(event.value as string | undefined)}
          className="w-15rem"
          aria-label="Sort documents"
        />
        <span>{options.first} - {options.last} of {options.totalRecords}</span>
      </div>
    ),
  };

  const actionMenuItems: MenuItem[] = [
    { icon: 'pi pi-upload', label: 'New Document', url: '/documents/new' },
    { separator: true },
    { icon: 'pi pi-plus-circle', label: 'Add to Cabinet', command: () => { openAddToCabinetDialog(); }, disabled: selectedIds.size === 0 },
    { icon: 'pi pi-minus-circle', label: 'Remove Cabinet', command: () => { openRemoveFromCabinetDialog(); }, disabled: selectedIds.size === 0 },
    { separator: true },
    { icon: 'pi pi-plus-circle', label: 'Add Tag', command: () => { openAddTagDialog(); }, disabled: selectedIds.size === 0 },
    { icon: 'pi pi-minus-circle', label: 'Remove Tag', command: () => { openRemoveTagDialog(); }, disabled: selectedIds.size === 0 },
    { separator: true },
    { icon: 'pi pi-trash', label: 'Delete Documents', command: () => { confirmDeleteSelectedDocuments(); }, disabled: selectedIds.size === 0 },
  ];

  return (
    <>
    <Menu model={actionMenuItems} popup ref={actionMenu} popupAlignment="right" id="action_menu"/>
    <Button
        label="Actions" className="mt-3 mr-5" style={{float: 'right'}} size="small" raised
        onClick={(event) => actionMenu.current?.toggle(event)} aria-controls="action_menu" aria-haspopup
      />

    <Card title="Documents">
      <DataView value={data?.items ?? []}
          loading={isPending || isFetching}
          onPage={onPage} paginator={true} first={0} rows={data?.per_page} totalRecords={data?.total}
          paginatorTemplate={paginatorTemplate}
          paginatorPosition="both"
          rowsPerPageOptions={[10, 20, 50, 100]}
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
    <Dialog
      header="Add to Cabinet"
      visible={addToCabinetVisible}
      onHide={closeAddToCabinetDialog}
      style={{ width: '90vw', maxWidth: '520px' }}
      dismissableMask={true}
      footer={(
        <div className="flex justify-content-end gap-2">
          <Button label="Cancel" type="button" severity="secondary" icon="pi pi-times" onClick={closeAddToCabinetDialog} />
          <Button
            label="Save"
            type="button"
            icon="pi pi-check"
            onClick={() => void saveAddToCabinet()}
            disabled={!selectedCabinetId || !selectedIds.size || saveCabinetDocument.isPending}
          />
        </div>
      )}
    >
      <div className="grid p-fluid">
        <div className="col-12">
          <label htmlFor="cabinet_id" className="font-medium mb-2 block">Cabinet</label>
          <Dropdown
            id="cabinet_id"
            value={selectedCabinetId}
            onChange={(event) => setSelectedCabinetId(event.value as number)}
            optionLabel="displayName"
            optionValue="id"
            placeholder="Select a cabinet"
            options={cabinetOptions?.items ?? []}
            loading={isCabinetsPending || isCabinetsFetching}
            className="w-full"
          />
        </div>
      </div>
    </Dialog>
    <Dialog
      header="Remove from Cabinet"
      visible={removeFromCabinetVisible}
      onHide={closeRemoveFromCabinetDialog}
      style={{ width: '90vw', maxWidth: '520px' }}
      dismissableMask={true}
      footer={(
        <div className="flex justify-content-end gap-2">
          <Button label="Cancel" type="button" severity="secondary" icon="pi pi-times" onClick={closeRemoveFromCabinetDialog} />
          <Button
            label="Remove"
            type="button"
            severity="danger"
            icon="pi pi-minus-circle"
            onClick={() => void saveRemoveFromCabinet()}
            disabled={!removeCabinetId || !selectedIds.size || removeCabinetDocument.isPending}
          />
        </div>
      )}
    >
      <div className="grid p-fluid">
        <div className="col-12">
          <label htmlFor="remove_cabinet_id" className="font-medium mb-2 block">Cabinet</label>
          <Dropdown
            id="remove_cabinet_id"
            value={removeCabinetId}
            onChange={(event) => setRemoveCabinetId(event.value as number)}
            optionLabel="displayName"
            optionValue="id"
            placeholder="Select a cabinet"
            options={cabinetOptions?.items ?? []}
            loading={isCabinetsPending || isCabinetsFetching}
            className="w-full"
          />
        </div>
      </div>
    </Dialog>
    <Dialog
      header="Add Tag"
      visible={addTagVisible}
      onHide={closeAddTagDialog}
      style={{ width: '90vw', maxWidth: '520px' }}
      dismissableMask={true}
      footer={(
        <div className="flex justify-content-end gap-2">
          <Button label="Cancel" type="button" severity="secondary" icon="pi pi-times" onClick={closeAddTagDialog} />
          <Button
            label="Save"
            type="button"
            icon="pi pi-check"
            onClick={() => void saveAddTag()}
            disabled={!selectedTagId || !selectedIds.size || saveTagDocument.isPending}
          />
        </div>
      )}
    >
      <div className="grid p-fluid">
        <div className="col-12">
          <label htmlFor="tag_id" className="font-medium mb-2 block">Tag</label>
          <Dropdown
            id="tag_id"
            value={selectedTagId}
            onChange={(event) => setSelectedTagId(event.value as number)}
            optionLabel="name"
            optionValue="id"
            placeholder="Select a tag"
            options={tagOptions?.items ?? []}
            loading={isTagsPending || isTagsFetching}
            className="w-full"
          />
        </div>
      </div>
    </Dialog>
    <Dialog
      header="Remove Tag"
      visible={removeTagVisible}
      onHide={closeRemoveTagDialog}
      style={{ width: '90vw', maxWidth: '520px' }}
      dismissableMask={true}
      footer={(
        <div className="flex justify-content-end gap-2">
          <Button label="Cancel" type="button" severity="secondary" icon="pi pi-times" onClick={closeRemoveTagDialog} />
          <Button
            label="Remove"
            type="button"
            severity="danger"
            icon="pi pi-minus-circle"
            onClick={() => void saveRemoveTag()}
            disabled={!removeTagId || !selectedIds.size || removeTagDocument.isPending}
          />
        </div>
      )}
    >
      <div className="grid p-fluid">
        <div className="col-12">
          <label htmlFor="remove_tag_id" className="font-medium mb-2 block">Tag</label>
          <Dropdown
            id="remove_tag_id"
            value={removeTagId}
            onChange={(event) => setRemoveTagId(event.value as number)}
            optionLabel="name"
            optionValue="id"
            placeholder="Select a tag"
            options={tagOptions?.items ?? []}
            loading={isTagsPending || isTagsFetching}
            className="w-full"
          />
        </div>
      </div>
    </Dialog>
    <ConfirmDialog />
    </>
  );
}

export default function UploadDocument() {
    const toast = useRef<Toast>(null);
    const [totalSize, setTotalSize] = useState(0);
    const fileUploadRef = useRef<FileUpload>(null);
    const [title, setTitle] = useState('');
    const [documentTypeId, setDocumentTypeId] = useState<number | null>(null);
    const { data: documentTypes, isPending: isDocumentTypesPending, isFetching: isDocumentTypesFetching } = useDocumentTypes({ page: 1, per_page: 200 });
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
