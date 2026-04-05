import { useMemo } from 'react';
import { Link } from 'react-router-dom';

import { useQueries } from '@tanstack/react-query';
import { Card } from 'primereact/card';
import { Column } from 'primereact/column';
import { DataTable } from 'primereact/datatable';
import { Message } from 'primereact/message';

import { DocumentViewLayout } from '../components/DocumentViewLayout';
import { type DocumentIndexValue } from '../models/documentIndex';
import { useDocumentIndexes } from '../queries/useDocumentIndexes';
import { useDocument } from '../queries/useDocuments';
import { useDocumentIndexMemberships } from '../queries/useDocumentIndexValues';
import { useId } from '../util';
import { apiFetch } from '../api';

type MembershipRow = {
  id: number;
  document_index_id: number;
  indexName: string;
  path: string;
};

export function ListDocumentIndexMembership() {
  const documentId = useId('id');
  const { data: document, isLoading: isDocumentLoading, isError: isDocumentError, error: documentError } = useDocument(documentId);
  const { data: memberships, isLoading: isMembershipsLoading, isError: isMembershipsError, error: membershipsError } = useDocumentIndexMemberships(documentId);
  const { data: indexList, isLoading: isIndexesLoading, isError: isIndexesError, error: indexesError } = useDocumentIndexes({ page: 1, per_page: 200 });

  const ancestorQueries = useQueries({
    queries: (memberships ?? []).map((membership) => ({
      queryKey: ['documentIndexValue', 'ancestors', membership.document_index_id, membership.id],
      queryFn: () => apiFetch<DocumentIndexValue[]>(`api/v1/document-indexes/${membership.document_index_id}/values/${membership.id}/ancestors`),
      enabled: true,
    })),
  });

  const indexNamesById = useMemo(() => {
    return Object.fromEntries((indexList?.items ?? []).map((item) => [item.id, item.name]));
  }, [indexList?.items]);

  const rows = useMemo<MembershipRow[]>(() => {
    return (memberships ?? []).map((membership, index) => {
      const ancestorsQuery = ancestorQueries[index];
      const ancestors = ancestorsQuery?.data ?? [];
      const path = ancestorsQuery?.isSuccess && ancestors.length > 0
        ? ancestors.map((item) => item.value).join(' / ')
        : membership.value;

      return {
        id: membership.id,
        document_index_id: membership.document_index_id,
        indexName: indexNamesById[membership.document_index_id] ?? `Index #${membership.document_index_id}`,
        path,
      };
    });
  }, [ancestorQueries, indexNamesById, memberships]);

  const indexTemplate = (row: MembershipRow) => (
    <Link className="title" to={`/indexes/${row.document_index_id}/values`}>
      {row.indexName}
    </Link>
  );

  const pathTemplate = (row: MembershipRow) => (
    <Link className="title" to={`/indexes/${row.document_index_id}/values/${row.id}/documents`}>
      {row.path}
    </Link>
  );

  if (isDocumentError) {
    return <Message severity="error" text={documentError.message} />;
  }

  if (isDocumentLoading) {
    return <div>Loading</div>;
  }

  return (
    <DocumentViewLayout documentId={documentId}>
      <Card title={`Document Indexes${document?.title ? `: ${document.title}` : ''}`}>
        {isMembershipsError && <Message severity="error" text={membershipsError.message} />}
        {isIndexesError && <Message severity="error" text={indexesError.message} />}

        {!isMembershipsError && !isMembershipsLoading && rows.length === 0 && (
          <Message severity="info" text="This document does not belong to any indexes." />
        )}

        {rows.length > 0 && (
          <DataTable value={rows} loading={isMembershipsLoading || isIndexesLoading}>
            <Column field="indexName" header="Index" body={indexTemplate} />
            <Column field="path" header="Path" body={pathTemplate} />
          </DataTable>
        )}
      </Card>
    </DocumentViewLayout>
  );
}
