import { useMemo, useState } from 'react';

import { Card } from 'primereact/card';
import { Divider } from 'primereact/divider';
import { Dropdown } from 'primereact/dropdown';
import { Message } from 'primereact/message';

import { DocumentViewLayout } from '../components/DocumentViewLayout';
import { useId } from '../util';
import { useDocument } from '../queries/useDocuments';
import { useDocumentFiles, useDocumentFilePages } from '../queries/useDocumentFiles';

export function ListDocumentFilePageTextContent() {
  const documentId = useId('id');
  const { data: document, isLoading: isDocumentLoading, isError: isDocumentError, error: documentError } = useDocument(documentId);
  const { data: files, isLoading: isFilesLoading, isError: isFilesError, error: filesError } = useDocumentFiles(documentId);
  const [selectedFileId, setSelectedFileId] = useState<number | null>(null);

  const effectiveFileId = useMemo(() => {
    if (!files?.length) return null;
    if (!selectedFileId) return files[0].id;
    if (!files.some((file) => file.id === selectedFileId)) return files[0].id;
    return selectedFileId;
  }, [files, selectedFileId]);

  const { data: pages, isLoading: isPagesLoading, isError: isPagesError, error: pagesError } = useDocumentFilePages(
    documentId,
    effectiveFileId ?? 0
  );

  const fileOptions = useMemo(
    () => (files ?? []).map((file) => ({ label: file.filename, value: file.id })),
    [files]
  );

  if (isDocumentError) {
    return <Message severity="error" text={documentError.message} />;
  }

  if (isDocumentLoading) {
    return <div>Loading</div>;
  }

  return (
    <DocumentViewLayout documentId={documentId}>
      <Card title={`Document Text${document?.title ? `: ${document.title}` : ''}`}>
        {isFilesError && <Message severity="error" text={filesError.message} />}
        {isPagesError && <Message severity="error" text={pagesError.message} />}

        <div className="flex flex-column gap-3">
          <div className="flex flex-wrap gap-2 align-items-center">
            <span className="font-medium">File</span>
            <Dropdown
              value={effectiveFileId}
              onChange={(event) => setSelectedFileId(event.value as number)}
              options={fileOptions}
              placeholder={isFilesLoading ? 'Loading files...' : 'Select a file'}
              className="w-full md:w-20rem"
              loading={isFilesLoading}
            />
          </div>

          {!isFilesLoading && !files?.length && (
            <Message severity="info" text="No files available for this document." />
          )}

          {effectiveFileId && (
            <div className="flex flex-column gap-3">
              {isPagesLoading && <div>Loading</div>}
              {!isPagesLoading && (pages ?? []).length === 0 && (
                <Message severity="info" text="No text content available for this file." />
              )}
              {(pages ?? []).map((page) => (
                <div key={`${page.document_file_id}-${page.page_number}`}>
                  <Divider align="center">Page {page.page_number}</Divider>
                  <pre className="aut-document-text-content">
                    {page.text_content ?? ''}
                  </pre>
                </div>
              ))}
            </div>
          )}
        </div>
      </Card>
    </DocumentViewLayout>
  );
}
