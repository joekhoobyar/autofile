import { useEffect, useRef, useState } from 'react';
import { Link } from 'react-router-dom';

import { Card } from 'primereact/card';
import { Checkbox } from 'primereact/checkbox';
import { DataView, DataViewLayoutOptions, type DataViewPageEvent } from 'primereact/dataview';
import { Dropdown } from 'primereact/dropdown';
import { Dialog } from 'primereact/dialog';
import { classNames } from 'primereact/utils';
import { format } from "date-fns";
import { confirmDialog, ConfirmDialog } from 'primereact/confirmdialog';

import { type ListParams } from '../api';
import { useDeleteDocument, useDocuments, useDocumentThumbnail, useRemoveCabinetDocument, useSaveCabinetDocument } from '../queries/useDocuments';
import { type Document } from '../models/document';
import { useMetadataTypesMap } from '../queries/useMetadataTypes';
import { useDocumentTypesMap } from '../queries/useDocumentTypes';
import { Menu } from 'primereact/menu';
import { Button } from 'primereact/button';
import type { MenuItem } from 'primereact/menuitem';
import { useCabinets } from '../queries/useCabinets';
import { MAX_CABINETS } from '../models/cabinet';

type DocumentListItemProps = {
  doc: Readonly<Document>;
  index: number;
  onImageClick: (src: string | undefined, title: string) => void;
  selected: boolean;
  onSelectionChange: (id: number, checked: boolean) => void;
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
  const [loadedThumbnailSrc, setLoadedThumbnailSrc] = useState<string | undefined>(undefined);

  useEffect(() => {
    if (!src) return;
    let cancelled = false;
    const img = new Image();
    img.onload = () => {
      if (!cancelled) {
        setLoadedThumbnailSrc(src);
      }
    };
    img.onerror = () => {
      if (!cancelled) {
        setLoadedThumbnailSrc(undefined);
      }
    };
    img.src = src;
    return () => {
      cancelled = true;
    };
  }, [src]);

  return (
    <div className="aut-document-thumbnail-wrapper">
      <button type="button" onClick={() => onClick(src)}>
        {loadedThumbnailSrc === src && src ? (
          <img
            alt={alt}
            className={imgClassName}
            src={src}
            style={{ maxHeight: '200px', display: 'block' }}
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
function DocumentListItem({ doc, index, onImageClick, selected, onSelectionChange }: Readonly<DocumentListItemProps>) {
    const { data } = useDocumentThumbnail(doc.id);
    const { data: mdt } = useMetadataTypesMap('slug');
    const { data: ddt } = useDocumentTypesMap();

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
            <div className="flex flex-column align-items-center sm:align-items-start gap-3">
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
                {Object.entries(doc.metadata).map(([key, value]) => (
                  <li key={key}><span>{mdt?.[key].name ?? key}</span>: {value}</li>
                ))}
              </ul>
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
};

/**
 * Renders a document in grid layout for the DataView component.
 * 
 * @param doc document
 * @returns HTML element for a document in grid layout
 */
function DocumentGridItem({ doc, onImageClick, selected, onSelectionChange }: Readonly<DocumentGridItemProps>) {
    const { data } = useDocumentThumbnail(doc.id);
    const { data: mdt } = useMetadataTypesMap('slug');
    const { data: ddt } = useDocumentTypesMap();

    return (
      <div className="col-12 sm:col-6 lg:col-4 xl:col-2 p-2 aut-document-grid" key={doc.id}>
        <div className="border-1 surface-border surface-card border-round">
          <section className="flex flex-column aut-document">
            <header className="flex align-items-center gap-2">
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
                {Object.entries(doc.metadata).map(([key, value]) => (
                  <li key={key}><span>{mdt?.[key].name ?? key}</span>: {value}</li>
                ))}
              </ul>
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
  const [listParams, setListParams] = useState<ListParams>({
    page: 1,
    sf: 'created_at',
    sd: true,
  });
  const [layout, setLayout] = useState<'list' | 'grid'>('grid');
  const [previewVisible, setPreviewVisible] = useState(false);
  const [previewSrc, setPreviewSrc] = useState<string | undefined>(undefined);
  const [previewTitle, setPreviewTitle] = useState('');
  const [selectedIds, setSelectedIds] = useState<Set<number>>(new Set());
  const [addToCabinetVisible, setAddToCabinetVisible] = useState(false);
  const [selectedCabinetId, setSelectedCabinetId] = useState<number | null>(null);
  const [removeFromCabinetVisible, setRemoveFromCabinetVisible] = useState(false);
  const [removeCabinetId, setRemoveCabinetId] = useState<number | null>(null);
  const { isPending, data, isFetching } = useDocuments(listParams);
  const deleteDocument = useDeleteDocument();
  const saveCabinetDocument = useSaveCabinetDocument();
  const removeCabinetDocument = useRemoveCabinetDocument();
  const { data: cabinetOptions, isPending: isCabinetsPending, isFetching: isCabinetsFetching } = useCabinets({ page: 1, per_page: MAX_CABINETS });
  const actionMenu = useRef<Menu>(null);

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

  const itemTemplate = (doc: Document, layout: 'list' | 'grid', index: number) => {
    if (!doc)
      return;
    if (layout === 'list')
      return (
        <DocumentListItem
          doc={doc}
          index={index}
          onImageClick={openPreview}
          selected={selectedIds.has(doc.id)}
          onSelectionChange={handleSelectionChange}
        />
      );
    else if (layout === 'grid')
      return (
        <DocumentGridItem
          doc={doc}
          onImageClick={openPreview}
          selected={selectedIds.has(doc.id)}
          onSelectionChange={handleSelectionChange}
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
    <ConfirmDialog />
    </>
  );
}
