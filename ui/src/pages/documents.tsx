import { useState } from 'react';
import { Link } from 'react-router-dom';

import { Card } from 'primereact/card';
import { DataView, DataViewLayoutOptions, type DataViewPageEvent } from 'primereact/dataview';
import { classNames } from 'primereact/utils';

import { type ListParams } from '../api';
import { useDocuments, useDocumentThumbnail } from '../queries/useDocuments';
import { type Document } from '../models/document';

/**
 * Renders a document in list layout for the DataView component.
 * 
 * @param doc document
 * @param index index in the list
 * @returns HTML element for a document in list layout
 */
function DocumentListItem(doc: Readonly<Document>, index: number) {
    const { data } = useDocumentThumbnail(doc.id);

    return (
      <div className="col-12" key={doc.id}>
        <div className={classNames('flex flex-column xl:flex-row xl:align-items-start p-4 gap-4', { 'border-top-1 surface-border': index !== 0 })}>
          <img alt="Page 1" className="w-9 sm:w-16rem xl:w-10rem shadow-2 block xl:block mx-auto aut-document-thumbnail" src={data} style={{ maxHeight: '200px' }} />
          <div className="flex flex-column sm:flex-row justify-content-between align-items-center xl:align-items-start flex-1 gap-4">
              <div className="flex flex-column align-items-center sm:align-items-start gap-3">
                  <div className="text-2xl font-bold text-900">{doc.title}</div>
              </div>
          </div>
        </div>
      </div>
    );
}

/**
 * Renders a document in grid layout for the DataView component.
 * 
 * @param doc document
 * @returns HTML element for a document in grid layout
 */
function DocumentGridItem(doc: Readonly<Document>) {
    const { data } = useDocumentThumbnail(doc.id);

    return (
      <div className="col-12 sm:col-6 lg:col-4 xl:col-2 p-2 aut-document-grid" key={doc.id}>
        <div className="border-1 surface-border surface-card border-round">
          <section className="flex flex-column align-items-center aut-document">
            <header>{doc.title}</header>
            <aside>
              <img alt="Page 1" className="shadow-2 aut-document-thumbnail" src={data} style={{ maxHeight: '200px' }} />
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
  const [listParams, setListParams] = useState<ListParams>({});
  const [layout, setLayout] = useState<'list' | 'grid'>('grid');
  const { isPending, data, isFetching } = useDocuments(listParams);
/*
  const onSort = (event: DataTableStateEvent) => {
    setListParams({ ...listParams, sf: event.sortField as string, sd: event.sortOrder === -1 });
  };
*/

  const onPage = (event: DataViewPageEvent) => {
    setListParams({ ...listParams, page: event.page, per_page: event.rows });
  };

  const itemTemplate = (doc: Document, layout: 'list' | 'grid', index: number) => {
    if (!doc)
      return;
    if (layout === 'list')
      return DocumentListItem(doc, index);
    else if (layout === 'grid')
      return DocumentGridItem(doc);
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

  return (
    <>
    <Link to="new" style={{float: 'right', padding: '1.5rem'}}>New Document &raquo;</Link>
    <Card title="Documents">
      <DataView value={data?.items ?? []}
          loading={isPending || isFetching}
          onPage={onPage} paginator={true} first={0} rows={data?.per_page} totalRecords={data?.total}
          listTemplate={listTemplate} layout={layout} header={header()}
        />
    </Card>
    </>
  );
}
