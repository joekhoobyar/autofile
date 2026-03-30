import { useMemo, useState } from 'react';
import { useNavigate, useParams } from 'react-router-dom';

import { DataTable, type DataTableStateEvent } from 'primereact/datatable';
import { Column } from 'primereact/column';
import { Card } from 'primereact/card';
import { Message } from 'primereact/message';
import { Menu } from 'primereact/menu';
import type { MenuItem } from 'primereact/menuitem';

import { useDocumentIndex } from '../queries/useDocumentIndexes';
import { useDocumentIndexValues } from '../queries/useDocumentIndexValues';
import { type DocumentIndexValue, type DocumentIndexValueListParams } from '../models/documentIndex';

export function ListDocumentIndexValues() {
  const { documentIndexId: documentIndexIdParam } = useParams();
  const documentIndexId = Number(documentIndexIdParam);
  const navigate = useNavigate();
  const [listParams, setListParams] = useState<DocumentIndexValueListParams>({
    parent_id: 'null',
  });
  const [parentStack, setParentStack] = useState<Array<{ id: number; value: string }>>([]);
  const { data: documentIndex } = useDocumentIndex(documentIndexId);
  const { isPending, data, isFetching } = useDocumentIndexValues(documentIndexId, listParams);

  const onSort = (event: DataTableStateEvent) => {
    setListParams((prev) => ({ ...prev, sf: event.sortField, sd: event.sortOrder === -1 }));
  };

  const onPage = (event: DataTableStateEvent) => {
    setListParams((prev) => ({ ...prev, page: event.page, per_page: event.rows }));
  };

  const valueTemplate = (row: DocumentIndexValue) => {
    return (
      <a
        className="title"
        href="#"
        onClick={(event) => {
          event.preventDefault();
          if (row.is_leaf) {
            navigate(`/indexes/${documentIndexId}/documents`);
            return;
          }
          setParentStack((prev) => [...prev, { id: row.id, value: row.value }]);
          setListParams((prev) => ({ ...prev, parent_id: row.id, page: 0 }));
        }}
      >
        {row.value}
      </a>
    );
  };

  const parentMenuItems = useMemo<MenuItem[]>(() => {
    const rootItem: MenuItem = {
      label: documentIndex?.name ?? 'Document Index',
      icon: 'pi pi-folder',
      command: () => {
        setParentStack([]);
        setListParams((prev) => ({ ...prev, parent_id: 'null', page: 0 }));
      },
    };

    const stackItems = parentStack.map((item, index) => ({
      label: item.value,
      icon: 'pi pi-folder',
      command: () => {
        setParentStack(parentStack.slice(0, index + 1));
        setListParams((prev) => ({ ...prev, parent_id: item.id, page: 0 }));
      },
    }));
    return [rootItem, ...stackItems];
  }, [documentIndex?.name, parentStack]);

  if (!documentIndexIdParam || Number.isNaN(documentIndexId))
    return <Message severity="error" text="Missing or invalid document index ID" />;

  return (
    <div className="flex gap-3 align-items-start">
      <Menu model={parentMenuItems} style={{ minWidth: '14rem' }} />
      <Card title="Document Index Values" className="flex-1">
        <DataTable lazy value={data?.items}
            onPage={onPage} paginator={true} first={0} rows={data?.per_page} totalRecords={data?.total}
            loading={isPending || isFetching}
            onSort={onSort} sortField={listParams.sf} sortOrder={listParams.sd === true ? -1 : 1}
          >
          <Column field="value" header="Value" body={valueTemplate} sortable></Column>
        </DataTable>
      </Card>
    </div>
  );
}
