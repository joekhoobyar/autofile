import { useState } from 'react';
import { Link } from 'react-router-dom';

import { Card } from 'primereact/card';
import { DataView, DataViewLayoutOptions, type DataViewPageEvent } from 'primereact/dataview';
import { classNames } from 'primereact/utils';

import type { ListParams } from '../api';
import { useDocuments } from '../queries/useDocuments';
import { type Document } from '../models/document';

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

  const listItem = (doc: Document, index: number) => {
    return (
      <div className="col-12" key={doc.id}>
        <div className={classNames('flex flex-column xl:flex-row xl:align-items-start p-4 gap-4', { 'border-top-1 surface-border': index !== 0 })}>
          <img className="w-9 sm:w-16rem xl:w-10rem shadow-2 block xl:block mx-auto border-round" src={`https://primefaces.org/cdn/primereact/images/product/gaming-set.jpg`} />
          <div className="flex flex-column sm:flex-row justify-content-between align-items-center xl:align-items-start flex-1 gap-4">
              <div className="flex flex-column align-items-center sm:align-items-start gap-3">
                  <div className="text-2xl font-bold text-900">{doc.title}</div>
              </div>
          </div>
        </div>
      </div>
    );
  };

  const gridItem = (doc: Document) => {
    return (
      <div className="col-12 sm:col-6 lg:col-4 xl:col-2 p-2" key={doc.id}>
        <div className="p-4 border-1 surface-border surface-card border-round">
          <div className="flex flex-column align-items-center gap-3 py-5">
            <div className="text-2xl font-bold">{doc.title}</div>
            <img className="w-9 shadow-2 border-round" src={`https://primefaces.org/cdn/primereact/images/product/gaming-set.jpg`} />
          </div>
        </div>
      </div>
    );
};

  const itemTemplate = (doc: Document, layout: 'list' | 'grid', index: number) => {
    if (!doc)
      return;
    if (layout === 'list')
      return listItem(doc, index);
    else if (layout === 'grid')
      return gridItem(doc);
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
    <Card title="Document">
      <DataView value={data?.items ?? []}
          loading={isPending || isFetching}
          onPage={onPage} paginator={true} first={0} rows={data?.per_page} totalRecords={data?.total}
          listTemplate={listTemplate} layout={layout} header={header()}
        />
    </Card>
    </>
  );
}
