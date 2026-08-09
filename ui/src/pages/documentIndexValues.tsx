import { useMemo } from 'react';
import { useNavigate, useParams } from 'react-router-dom';

import { DataTable, type DataTableStateEvent } from 'primereact/datatable';
import { Column } from 'primereact/column';
import { Card } from 'primereact/card';
import { Message } from 'primereact/message';
import { Menu } from 'primereact/menu';
import type { MenuItem } from 'primereact/menuitem';

import { useDocumentIndex } from '../queries/useDocumentIndexes';
import { useDocumentIndexValueAncestors, useDocumentIndexValues } from '../queries/useDocumentIndexValues';
import { type DocumentIndexValue, type DocumentIndexValueListParams } from '../models/documentIndex';
import { useHashListParams } from '../util/listParamsHash';

const DOCUMENT_INDEX_VALUE_LIST_DEFAULT_PARAMS: DocumentIndexValueListParams = {};

export function ListDocumentIndexValues() {
  const { documentIndexId: documentIndexIdParam, documentIndexValueId: documentIndexValueIdParam } = useParams();
  const documentIndexId = Number(documentIndexIdParam);
  const parsedDocumentIndexValueId = documentIndexValueIdParam ? Number(documentIndexValueIdParam) : undefined;
  const documentIndexValueId = parsedDocumentIndexValueId && !Number.isNaN(parsedDocumentIndexValueId)
    ? parsedDocumentIndexValueId
    : undefined;
  const navigate = useNavigate();
  const { listParams, updateListParams } = useHashListParams(DOCUMENT_INDEX_VALUE_LIST_DEFAULT_PARAMS);
  const { data: documentIndex } = useDocumentIndex(documentIndexId);
  const effectiveListParams: DocumentIndexValueListParams = {
    ...listParams,
    parent_id: documentIndexValueId ?? 'null',
  };
  const { isPending, data, isFetching } = useDocumentIndexValues(documentIndexId, effectiveListParams);
  const { data: indexAncestors } = useDocumentIndexValueAncestors(documentIndexId, documentIndexValueId ?? 0);

  const onSort = (event: DataTableStateEvent) => {
    updateListParams({ ...listParams, sf: event.sortField, sd: event.sortOrder === -1, page: 1 });
  };

  const onPage = (event: DataTableStateEvent) => {
    updateListParams({ ...listParams, page: (event.page ?? 0) + 1, per_page: event.rows });
  };

  const valueTemplate = (row: DocumentIndexValue) => {
    return (
      <a
        className="title"
        href="#"
        onClick={(event) => {
          event.preventDefault();
          if (row.is_leaf) {
            navigate(`/indexes/${documentIndexId}/values/${row.id}/documents`);
            return;
          }
          navigate(`/indexes/${documentIndexId}/values/${row.id}`);
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
        navigate(`/indexes/${documentIndexId}/values`);
      },
    };

    const stackItems = (indexAncestors ?? []).map((item) => ({
      label: item.value,
      icon: 'pi pi-folder',
      command: () => {
        navigate(`/indexes/${documentIndexId}/values/${item.id}`);
      },
    }));
    return [rootItem, ...stackItems];
  }, [documentIndex?.name, documentIndexId, indexAncestors, navigate]);

  if (!documentIndexIdParam || Number.isNaN(documentIndexId))
    return <Message severity="error" text="Missing or invalid document index ID" />;

  return (
    <div className="flex gap-3 align-items-start">
      <Menu model={parentMenuItems} style={{ minWidth: '14rem' }} />
      <Card title="Document Index Values" className="flex-1">
        <DataTable lazy value={data?.items}
            onPage={onPage}
            paginator={true}
            first={Math.max(((data?.page ?? listParams.page ?? 1) - 1) * (data?.per_page ?? listParams.per_page ?? 0), 0)}
            rows={data?.per_page ?? listParams.per_page}
            totalRecords={data?.total}
            loading={isPending || isFetching}
            onSort={onSort} sortField={listParams.sf} sortOrder={listParams.sd === true ? -1 : 1}
          >
          <Column field="value" header="Value" body={valueTemplate} sortable></Column>
          <Column field="document_count" header="Documents"></Column>
        </DataTable>
      </Card>
    </div>
  );
}
