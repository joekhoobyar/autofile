import { useMemo, useState } from 'react';

import { Card } from 'primereact/card';
import { Divider } from 'primereact/divider';
import { Dropdown } from 'primereact/dropdown';
import { Message } from 'primereact/message';

import { DocumentViewLayout } from '../components/DocumentViewLayout';
import { useId } from '../util';
import { useDocument } from '../queries/useDocuments';
import {
  useDocumentFiles,
  useDocumentFileOcrPages,
  useDocumentFilePages,
  useDocumentFilePageImage,
} from '../queries/useDocumentFiles';

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

export function ListDocumentFilePageOcrContent() {
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

  const { data: pages, isLoading: isPagesLoading, isError: isPagesError, error: pagesError } = useDocumentFileOcrPages(
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
      <Card title={`Document OCR${document?.title ? `: ${document.title}` : ''}`}>
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
                <Message severity="info" text="No OCR content available for this file." />
              )}
              {(pages ?? []).map((page) => (
                <div key={`${page.document_file_id}-${page.page_number}`}>
                  <Divider align="center">Page {page.page_number}</Divider>
                  <pre className="aut-document-text-content">
                    {page.ocr_content ?? ''}
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

type PageImageItemProps = {
  documentId: number;
  documentFileId: number;
  pageNumber: number;
};

function PageImageItem({ documentId, documentFileId, pageNumber }: Readonly<PageImageItemProps>) {
  const { data: imageUrl, isLoading, isError, error } = useDocumentFilePageImage(
    documentId,
    documentFileId,
    pageNumber
  );

  return (
    <div>
      <Divider align="center">Page {pageNumber}</Divider>
      {isError && <Message severity="error" text={error.message} />}
      {isLoading && <div>Loading page {pageNumber}</div>}
      {!isLoading && !isError && !imageUrl && (
        <Message severity="info" text="Page image not available yet." />
      )}
      {!isLoading && !isError && imageUrl && (
        <img
          src={imageUrl}
          alt={`Page ${pageNumber}`}
          style={{ width: '100%', height: 'auto', display: 'block' }}
        />
      )}
    </div>
  );
}

export function DocumentFilePagePreview() {
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

  const effectiveFile = useMemo(
    () => (files ?? []).find((file) => file.id === effectiveFileId) ?? null,
    [effectiveFileId, files]
  );

  const fileOptions = useMemo(
    () => (files ?? []).map((file) => ({ label: file.filename, value: file.id })),
    [files]
  );

  const pageNumbers = useMemo(() => {
    if (!effectiveFile || effectiveFile.pages <= 0) return [];
    return Array.from({ length: effectiveFile.pages }, (_, index) => index + 1);
  }, [effectiveFile]);

  if (isDocumentError) {
    return <Message severity="error" text={documentError.message} />;
  }

  if (isDocumentLoading) {
    return <div>Loading</div>;
  }

  return (
    <DocumentViewLayout documentId={documentId}>
      <Card title={`Document Preview${document?.title ? `: ${document.title}` : ''}`}>
        {isFilesError && <Message severity="error" text={filesError.message} />}

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
              {!isFilesLoading && pageNumbers.length === 0 && (
                <Message severity="info" text="No pages available for this file." />
              )}
              {pageNumbers.map((pageNumber) => (
                <PageImageItem
                  key={`${effectiveFileId}-${pageNumber}`}
                  documentId={documentId}
                  documentFileId={effectiveFileId}
                  pageNumber={pageNumber}
                />
              ))}
            </div>
          )}
        </div>
      </Card>
    </DocumentViewLayout>
  );
}
