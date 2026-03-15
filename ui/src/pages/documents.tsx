import { useEffect, useState } from 'react';
import { Link } from 'react-router-dom';

import { Card } from 'primereact/card';
import { DataView, DataViewLayoutOptions, type DataViewPageEvent } from 'primereact/dataview';
import { Dropdown } from 'primereact/dropdown';
import { Dialog } from 'primereact/dialog';
import { classNames } from 'primereact/utils';
import { format } from "date-fns";

import { type ListParams } from '../api';
import { useDocuments, useDocumentThumbnail } from '../queries/useDocuments';
import { type Document } from '../models/document';
import { useMetadataTypesMap } from '../queries/useMetadataTypes';
import { useDocumentTypesMap } from '../queries/useDocumentTypes';

type DocumentListItemProps = {
  doc: Readonly<Document>;
  index: number;
  onImageClick: (src: string | undefined, title: string) => void;
};

/**
 * Renders a document in list layout for the DataView component.
 * 
 * @param doc document
 * @param index index in the list
 * @returns HTML element for a document in list layout
 */
function DocumentListItem({ doc, index, onImageClick }: Readonly<DocumentListItemProps>) {
    const { data } = useDocumentThumbnail(doc.id);
    const { data: mdt } = useMetadataTypesMap('slug');
    const { data: ddt } = useDocumentTypesMap();
    const [loadedThumbnailSrc, setLoadedThumbnailSrc] = useState<string | undefined>(undefined);

    useEffect(() => {
      if (!data) return;
      let cancelled = false;
      const img = new Image();
      img.onload = () => {
        if (!cancelled) {
          setLoadedThumbnailSrc(data);
        }
      };
      img.onerror = () => {
        if (!cancelled) {
          setLoadedThumbnailSrc(undefined);
        }
      };
      img.src = data;
      return () => {
        cancelled = true;
      };
    }, [data]);

    return (
      <div className="col-12 aut-document-list" key={doc.id}>
        <div className={classNames('flex flex-column xl:flex-row xl:align-items-start p-4 gap-4', { 'border-top-1 surface-border': index !== 0 })}>
          <div className="aut-document-thumbnail-wrapper">
            <button type="button"
              onClick={() => onImageClick(data, doc.title)}
            >
              {loadedThumbnailSrc === data && data ? (
                <img
                  alt="Page 1"
                  className="w-9 sm:w-16rem xl:w-10rem block xl:block mx-auto aut-document-thumbnail"
                  src={data}
                  style={{ maxHeight: '200px', display: 'block' }}
                />
              ) : (
                <div
                  className="w-9 sm:w-16rem xl:w-10rem block xl:block mx-auto aut-document-thumbnail"
                  style={{ maxHeight: '200px', height: '200px', display: 'flex', alignItems: 'center', justifyContent: 'center' }}
                  aria-label="Loading thumbnail"
                >
                  <span className="pi pi-spin pi-spinner" aria-hidden="true" />
                </div>
              )}
            </button>
          </div>
          <section className="flex flex-column sm:flex-row justify-content-between align-items-center xl:align-items-start flex-1 gap-4 aut-document">
            <div className="flex flex-column align-items-center sm:align-items-start gap-3">
              <header>
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
};

/**
 * Renders a document in grid layout for the DataView component.
 * 
 * @param doc document
 * @returns HTML element for a document in grid layout
 */
function DocumentGridItem({ doc, onImageClick }: Readonly<DocumentGridItemProps>) {
    const { data } = useDocumentThumbnail(doc.id);
    const { data: mdt } = useMetadataTypesMap('slug');
    const { data: ddt } = useDocumentTypesMap();
    const [loadedThumbnailSrc, setLoadedThumbnailSrc] = useState<string | undefined>(undefined);

    useEffect(() => {
      if (!data) return;
      let cancelled = false;
      const img = new Image();
      img.onload = () => {
        if (!cancelled) {
          setLoadedThumbnailSrc(data);
        }
      };
      img.onerror = () => {
        if (!cancelled) {
          setLoadedThumbnailSrc(undefined);
        }
      };
      img.src = data;
      return () => {
        cancelled = true;
      };
    }, [data]);

    return (
      <div className="col-12 sm:col-6 lg:col-4 xl:col-2 p-2 aut-document-grid" key={doc.id}>
        <div className="border-1 surface-border surface-card border-round">
          <section className="flex flex-column aut-document">
            <header>
              <Link to={`${doc.id}/metadata`}>{doc.title}</Link>
            </header>
            <aside>
              <div className="aut-document-thumbnail-wrapper">
                <button
                  type="button"
                  onClick={() => onImageClick(data, doc.title)}
                  style={{ border: 'none', background: 'transparent', padding: 0, cursor: 'pointer' }}
                >
                  {loadedThumbnailSrc === data && data ? (
                    <img
                      alt="Page 1"
                      className="aut-document-thumbnail"
                      src={data}
                      style={{ maxHeight: '200px', display: 'block' }}
                    />
                  ) : (
                    <div
                      className="aut-document-thumbnail"
                      style={{ maxHeight: '200px', height: '200px', display: 'flex', alignItems: 'center', justifyContent: 'center' }}
                      aria-label="Loading thumbnail"
                    >
                      <span className="pi pi-spin pi-spinner" aria-hidden="true" />
                    </div>
                  )}
                </button>
              </div>
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
  const { isPending, data, isFetching } = useDocuments(listParams);

  const sortOptions = [
    { label: 'ID (Ascending)', value: 'id:asc' },
    { label: 'ID (Descending)', value: 'id:desc' },
    { label: 'Title (A-Z)', value: 'title:asc' },
    { label: 'Title (Z-A)', value: 'title:desc' },
    { label: 'Created (Oldest)', value: 'created_at:asc' },
    { label: 'Created (Newest)', value: 'created_at:desc' },
  ];

  const sortValue = listParams.sf
    ? `${listParams.sf}:${listParams.sd ? 'desc' : 'asc'}`
    : undefined;

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

  const itemTemplate = (doc: Document, layout: 'list' | 'grid', index: number) => {
    if (!doc)
      return;
    if (layout === 'list')
      return <DocumentListItem doc={doc} index={index} onImageClick={openPreview} />;
    else if (layout === 'grid')
      return <DocumentGridItem doc={doc} onImageClick={openPreview} />;
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

  return (
    <>
    <Link to="new" style={{float: 'right', padding: '1.5rem'}}>New Document &raquo;</Link>
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
    </>
  );
}
