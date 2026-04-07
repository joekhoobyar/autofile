import { useMemo, useState } from 'react';
import { useNavigate, useSearchParams } from 'react-router-dom';

import { Button } from 'primereact/button';
import { Card } from 'primereact/card';
import { DataView, DataViewLayoutOptions } from 'primereact/dataview';
import { Divider } from 'primereact/divider';
import { Dropdown } from 'primereact/dropdown';
import { Message } from 'primereact/message';
import { Skeleton } from 'primereact/skeleton';
import { classNames } from 'primereact/utils';
import { format } from 'date-fns';

import { API_HOST, HttpError, apiFetch } from '../api';
import { DocumentViewLayout } from '../components/DocumentViewLayout';
import { type DocumentFile } from '../models/documentFile';
import { useDocument } from '../queries/useDocuments';
import {
  useDocumentFiles,
  useDocumentFileOcrPages,
  useDocumentFilePageImage,
  useDocumentFilePages,
  useDocumentFileThumbnail,
} from '../queries/useDocumentFiles';
import { useId } from '../util';

type DocumentFileThumbnailProps = {
  documentId: number;
  file: DocumentFile;
};

type DocumentFileListItemProps = {
  documentId: number;
  file: DocumentFile;
  index: number;
  onOpenPreview: (fileId: number) => void;
  onDownload: (event: React.MouseEvent, file: DocumentFile) => void;
  isDownloading: boolean;
};

type DocumentFileGridItemProps = {
  documentId: number;
  file: DocumentFile;
  onOpenPreview: (fileId: number) => void;
  onDownload: (event: React.MouseEvent, file: DocumentFile) => void;
  isDownloading: boolean;
};

function formatBytes(size: number) {
  if (size < 1024) return `${size} B`;
  const units = ['KB', 'MB', 'GB', 'TB'];
  let value = size / 1024;
  let unitIndex = 0;
  while (value >= 1024 && unitIndex < units.length - 1) {
    value /= 1024;
    unitIndex += 1;
  }
  return `${value.toFixed(value >= 10 ? 0 : 1)} ${units[unitIndex]}`;
}

function useSelectedFileId(files: DocumentFile[] | undefined, initialFileId?: number) {
  const [selectedFileId, setSelectedFileId] = useState<number | null>(initialFileId ?? null);

  return useMemo(() => {
    const requestedFileId = selectedFileId ?? initialFileId ?? null;
    const effectiveFileId = (() => {
      if (!files?.length) return null;
      if (!requestedFileId) return files[0].id;
      if (!files.some((file) => file.id === requestedFileId)) return files[0].id;
      return requestedFileId;
    })();

    return {
      selectedFileId,
      setSelectedFileId,
      effectiveFileId,
      effectiveFile: (files ?? []).find((file) => file.id === effectiveFileId) ?? null,
    };
  }, [files, initialFileId, selectedFileId]);
}

function DocumentFileThumbnail({ documentId, file }: Readonly<DocumentFileThumbnailProps>) {
  const { data: thumbnailUrl } = useDocumentFileThumbnail(documentId, file.id);
  const [loadedSrc, setLoadedSrc] = useState<string | undefined>(undefined);
  const [errorSrc, setErrorSrc] = useState<string | undefined>(undefined);

  const isLoaded = !!thumbnailUrl && loadedSrc === thumbnailUrl;
  const hasError = !!thumbnailUrl && errorSrc === thumbnailUrl;

  return (
    <div className="aut-document-thumbnail-wrapper aut-document-file-thumbnail-wrapper">
      {thumbnailUrl && !hasError ? (
        <img
          alt={file.filename}
          className="aut-document-thumbnail aut-document-file-thumbnail"
          src={thumbnailUrl}
          onLoad={() => {
            setLoadedSrc(thumbnailUrl);
            setErrorSrc(undefined);
          }}
          onError={() => setErrorSrc(thumbnailUrl)}
          style={{ maxHeight: '200px', visibility: isLoaded ? 'visible' : 'hidden' }}
        />
      ) : (
        <div
          className="aut-document-file-thumbnail-placeholder"
          style={{ maxHeight: '200px', height: '200px', display: 'flex', alignItems: 'center', justifyContent: 'center' }}
        >
          <span className={classNames('pi', thumbnailUrl ? 'pi-spin pi-spinner' : 'pi-file')} aria-hidden="true" />
        </div>
      )}
    </div>
  );
}

function FileMetadata({ file }: Readonly<{ file: DocumentFile }>) {
  return (
    <ul className="aut-document-metadata aut-document-file-metadata">
      <li><span>Content Type</span>: {file.content_type ?? 'Unknown'}</li>
      <li><span>Size</span>: {formatBytes(file.size)}</li>
      <li><span>Pages</span>: {file.pages ?? 0}</li>
      <li><span>Created</span>: {format(new Date(file.created_at), 'MM/dd/yyyy HH:mm')}</li>
    </ul>
  );
}

function DownloadButton({ file, onDownload, isDownloading }: Readonly<{
  file: DocumentFile;
  onDownload: (event: React.MouseEvent, file: DocumentFile) => void;
  isDownloading: boolean;
}>) {
  return (
    <Button
      type="button"
      label={isDownloading ? 'Downloading' : 'Download'}
      icon={isDownloading ? 'pi pi-spin pi-spinner' : 'pi pi-download'}
      severity="contrast"
      outlined
      size="small"
      className="aut-document-file-download-button"
      aria-label={`Download ${file.filename}`}
      onClick={(event) => onDownload(event, file)}
      disabled={isDownloading}
    />
  );
}

function DocumentFileListItem({ documentId, file, index, onOpenPreview, onDownload, isDownloading }: Readonly<DocumentFileListItemProps>) {
  return (
    <div className="col-12 aut-document-list aut-document-file-list" key={file.id}>
      <div
        className={classNames('flex flex-column xl:flex-row xl:align-items-start p-4 gap-4 aut-document-file-card', {
          'border-top-1 surface-border': index !== 0,
        })}
        onClick={() => onOpenPreview(file.id)}
        role="button"
        tabIndex={0}
        onKeyDown={(event) => {
          if (event.key === 'Enter' || event.key === ' ') {
            event.preventDefault();
            onOpenPreview(file.id);
          }
        }}
      >
        <DocumentFileThumbnail documentId={documentId} file={file} />
        <section className="flex flex-column sm:flex-row justify-content-between align-items-center xl:align-items-start flex-1 gap-4 aut-document">
          <div className="flex flex-column align-items-center sm:align-items-start gap-3 aut-document-header">
            <header className="flex align-items-center justify-content-between gap-2 aut-document-header-row aut-document-file-header-row">
              <span className="aut-document-file-name">{file.filename}</span>
              <DownloadButton file={file} onDownload={onDownload} isDownloading={isDownloading} />
            </header>
          </div>
          <aside className="flex flex-column align-items-center sm:align-items-start">
            <FileMetadata file={file} />
          </aside>
        </section>
      </div>
    </div>
  );
}

function DocumentFileGridItem({ documentId, file, onOpenPreview, onDownload, isDownloading }: Readonly<DocumentFileGridItemProps>) {
  return (
    <div className="col-12 sm:col-6 lg:col-4 xl:col-3 p-2 aut-document-grid aut-document-file-grid" key={file.id}>
      <div
        className="border-1 surface-border surface-card aut-document-file-card"
        onClick={() => onOpenPreview(file.id)}
        role="button"
        tabIndex={0}
        onKeyDown={(event) => {
          if (event.key === 'Enter' || event.key === ' ') {
            event.preventDefault();
            onOpenPreview(file.id);
          }
        }}
      >
        <section className="flex flex-column aut-document w-full">
          <header className="flex align-items-center gap-2 aut-document-header aut-document-header-row aut-document-file-header-row">
            <span className="aut-document-file-name">{file.filename}</span>
          </header>
          <aside>
            <DocumentFileThumbnail documentId={documentId} file={file} />
            <FileMetadata file={file} />
            <div className="aut-document-file-grid-actions">
              <DownloadButton file={file} onDownload={onDownload} isDownloading={isDownloading} />
            </div>
          </aside>
        </section>
      </div>
    </div>
  );
}

export function ListDocumentFiles() {
  const documentId = useId('id');
  const navigate = useNavigate();
  const { data: document, isLoading: isDocumentLoading, isError: isDocumentError, error: documentError } = useDocument(documentId);
  const { data: files, isLoading: isFilesLoading, isError: isFilesError, error: filesError } = useDocumentFiles(documentId);
  const [layout, setLayout] = useState<'list' | 'grid'>('grid');
  const [downloadingIds, setDownloadingIds] = useState<Set<number>>(new Set());
  const [downloadError, setDownloadError] = useState<string | null>(null);

  const openPreview = (fileId: number) => {
    navigate(`/documents/${documentId}/preview?file_id=${fileId}`);
  };

  const handleDownload = async (event: React.MouseEvent, file: DocumentFile) => {
    event.preventDefault();
    event.stopPropagation();
    setDownloadError(null);
    setDownloadingIds((current) => new Set(current).add(file.id));

    try {
      const ticket = await apiFetch<{ url: string }>(`api/v1/documents/${documentId}/files/${file.id}/download-ticket`, {
        method: 'POST',
      });
      const link = window.document.createElement('a');
      link.href = `${API_HOST}/${ticket.url}`;
      link.download = file.filename;
      window.document.body.appendChild(link);
      link.click();
      link.remove();
    } catch (error) {
      setDownloadError(error instanceof HttpError ? error.message : 'Failed to download file');
    } finally {
      setDownloadingIds((current) => {
        const next = new Set(current);
        next.delete(file.id);
        return next;
      });
    }
  };

  const itemTemplate = (file: DocumentFile, currentLayout: 'list' | 'grid', index: number) => {
    if (!file) return null;
    if (currentLayout === 'list') {
      return (
        <DocumentFileListItem
          key={file.id}
          documentId={documentId}
          file={file}
          index={index}
          onOpenPreview={openPreview}
          onDownload={handleDownload}
          isDownloading={downloadingIds.has(file.id)}
        />
      );
    }

    return (
      <DocumentFileGridItem
        key={file.id}
        documentId={documentId}
        file={file}
        onOpenPreview={openPreview}
        onDownload={handleDownload}
        isDownloading={downloadingIds.has(file.id)}
      />
    );
  };

  const listTemplate = (items: DocumentFile[], currentLayout: 'list' | 'grid') => (
    <div className="grid grid-nogutter">{items.map((file, index) => itemTemplate(file, currentLayout, index))}</div>
  );

  const header = (
    <div className="flex justify-content-end">
      <DataViewLayoutOptions layout={layout} onChange={(event) => setLayout(event.value as 'list' | 'grid')} />
    </div>
  );

  if (isDocumentError) {
    return <Message severity="error" text={documentError.message} />;
  }

  if (isDocumentLoading) {
    return <div>Loading</div>;
  }

  return (
    <DocumentViewLayout documentId={documentId}>
      <Card title={`Document Files${document?.title ? `: ${document.title}` : ''}`}>
        {isFilesError && <Message severity="error" text={filesError.message} />}
        {downloadError && <Message severity="error" text={downloadError} />}
        {!isFilesLoading && !isFilesError && !files?.length && (
          <Message severity="info" text="No files available for this document." />
        )}
        {!!files?.length && (
          <DataView
            value={files}
            loading={isFilesLoading}
            listTemplate={listTemplate}
            layout={layout}
            header={header}
          />
        )}
      </Card>
    </DocumentViewLayout>
  );
}

export function ListDocumentFilePageTextContent() {
  const documentId = useId('id');
  const { data: document, isLoading: isDocumentLoading, isError: isDocumentError, error: documentError } = useDocument(documentId);
  const { data: files, isLoading: isFilesLoading, isError: isFilesError, error: filesError } = useDocumentFiles(documentId);
  const { effectiveFileId, setSelectedFileId } = useSelectedFileId(files);

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
  const { effectiveFileId, setSelectedFileId } = useSelectedFileId(files);

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
  const [loadedImageUrl, setLoadedImageUrl] = useState<string | undefined>(undefined);
  const isImageReady = !!imageUrl && loadedImageUrl === imageUrl;

  return (
    <div>
      <Divider align="center">Page {pageNumber}</Divider>
      {isError && <Message severity="error" text={error.message} />}
      {!isLoading && !isError && !imageUrl && (
        <Message severity="info" text="Page image not available yet." />
      )}
      {!isError && (isLoading || imageUrl) && (
        <div
          style={{
            position: 'relative',
            width: '100%',
            minHeight: '20rem',
            overflow: 'hidden',
          }}
        >
          {!isImageReady && (
            <div className="p-4">
                <div className="flex mb-3">
                    <Skeleton shape="circle" size="4rem" className="mr-2"></Skeleton>
                    <div>
                        <Skeleton width="10rem" className="mb-2"></Skeleton>
                        <Skeleton width="5rem" className="mb-2"></Skeleton>
                        <Skeleton height=".5rem"></Skeleton>
                    </div>
                </div>
                <Skeleton width="100%" height="30rem"></Skeleton>
                <div className="flex justify-content-between mt-3 mb-3">
                    <Skeleton width="4rem" height="2rem"></Skeleton>
                    <Skeleton width="4rem" height="2rem"></Skeleton>
                </div>
                <Skeleton width="100%" height="30rem"></Skeleton>
            </div>
          )}
          {imageUrl && (
            <img
              src={imageUrl}
              alt={`Page ${pageNumber}`}
              onLoad={() => setLoadedImageUrl(imageUrl)}
              style={{
                width: '100%',
                height: 'auto',
                display: 'block',
                opacity: isImageReady ? 1 : 0,
                transition: 'opacity 180ms ease',
              }}
            />
          )}
        </div>
      )}
    </div>
  );
}

export function DocumentFilePagePreview() {
  const documentId = useId('id');
  const [searchParams] = useSearchParams();
  const initialFileId = Number(searchParams.get('file_id'));
  const resolvedInitialFileId = Number.isNaN(initialFileId) ? undefined : initialFileId;
  const { data: document, isLoading: isDocumentLoading, isError: isDocumentError, error: documentError } = useDocument(documentId);
  const { data: files, isLoading: isFilesLoading, isError: isFilesError, error: filesError } = useDocumentFiles(documentId);
  const { effectiveFileId, effectiveFile, setSelectedFileId } = useSelectedFileId(files, resolvedInitialFileId);

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
