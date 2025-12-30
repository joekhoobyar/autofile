import { useState } from 'react';
import { DataTable, type DataTableStateEvent } from 'primereact/datatable';
import { Column } from 'primereact/column';
import { Card } from 'primereact/card';
import { Button } from 'primereact/button';
import { InputText } from 'primereact/inputtext';
import { FloatLabel } from 'primereact/floatlabel';

import type { ListParams } from '../api';
import { useMetadataTypes } from '../queries/useMetadataTypes';
import { type MetadataType } from '../models/metadataType';

export function ListMetadataTypes() {
  const [listParams, setListParams] = useState<ListParams>({});
  const { isPending, data, isFetching } = useMetadataTypes(listParams);

  const slugTemplate = (c: MetadataType) => {
    return (
      <a className="title" onClick={() => console.log(c.slug)}>{c.slug}</a>
    );
  }

  const nameTemplate = (c: MetadataType) => {
    return (
      <a className="title" onClick={() => console.log(c.name)}>{c.name}</a>
    );
  }

  const onSort = (event: DataTableStateEvent) => {
    setListParams({ ...listParams, sf: event.sortField as string, sd: event.sortOrder === -1 });
  };

  const onPage = (event: DataTableStateEvent) => {
    setListParams({ ...listParams, page: event.page, per_page: event.rows });
  };

  return (
    <Card title="Metadata Types">
      <DataTable lazy value={data?.items}
          onPage={onPage} paginator={true} first={0} rows={data?.per_page} totalRecords={data?.total}
          loading={isPending || isFetching}
          onSort={onSort} sortField={listParams.sf} sortOrder={listParams.sd===true ? -1 : 1}
        >
        <Column field="slug" header="Slug" body={slugTemplate} sortable></Column>
        <Column field="name" header="Name" body={nameTemplate} sortable></Column>
        <Column field="data_type" header="Data Type" sortable></Column>
        <Column field="description" header="Description" sortable></Column>
      </DataTable>
    </Card>
  );
}

export function NewMetadataType() {
  const [metadataType, ] = useState<Partial<MetadataType>>({});
  const footer = (
    <>
        <Button label="Save" icon="pi pi-check" />
        <Button label="Cancel" severity="secondary" icon="pi pi-times" style={{ marginLeft: '0.5em' }} />
    </>
  );

  return (
    <Card title="New Metadata Type" footer={footer}>
      <FloatLabel className="mb-4">
        <InputText id="slug" value={metadataType.slug}></InputText>
        <label htmlFor="slug">Slug</label>
      </FloatLabel>
      <FloatLabel className="mb-4">
        <InputText id="name" value={metadataType.name}></InputText>
        <label htmlFor="name">Name</label>
      </FloatLabel>
      <FloatLabel className="mb-4">
        <InputText id="description" value={metadataType.description}></InputText>
        <label htmlFor="description">Description</label>
      </FloatLabel>
    </Card>
  );
}
