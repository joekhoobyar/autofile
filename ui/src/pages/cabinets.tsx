import { useCabinets } from '../queries/useCabinets';
import { type Cabinet } from '../models/cabinet';
import { DataTable } from 'primereact/datatable';
import { Column } from 'primereact/column';
import { Card } from 'primereact/card';

export function ListCabinets() {
  const { isPending, data, isFetching } = useCabinets();

  const nameTemplate = (c: Cabinet) => {
    return (
      <a className="title" onClick={() => console.log(c.name)}>{c.name}</a>
    );
  }

  return (
    <Card title="Cabinets">
      <DataTable value={data} rows={data?.length} loading={isPending || isFetching}>
        <Column field="name" header="Name" body={nameTemplate} sortable></Column>
        <Column field="description" header="Description" sortable></Column>
      </DataTable>
    </Card>
  );
}
