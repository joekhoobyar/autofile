import { useMetadataTypes } from '../queries/useMetadataTypes';
import { type MetadataType } from '../models/metadataType';
import { DataTable } from 'primereact/datatable';
import { Column } from 'primereact/column';

export function ListMetadataTypes() {
  const { isPending, data, isFetching } = useMetadataTypes();

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

  return (
    <div className="card">
      <DataTable value={data?.items} rows={data?.items?.length} loading={isPending || isFetching}>
        <Column field="slug" header="Slug" body={slugTemplate} sortable></Column>
        <Column field="name" header="Name" body={nameTemplate} sortable></Column>
        <Column field="data_type" header="Data Type" sortable></Column>
        <Column field="description" header="Description" sortable></Column>
      </DataTable>
    </div>
  );
}
