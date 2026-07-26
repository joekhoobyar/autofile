import { useMemo, useRef, useState, type ReactNode } from 'react';
import { useNavigate, useSearchParams } from 'react-router-dom';
import { useQueryClient } from '@tanstack/react-query';

import { Button } from 'primereact/button';
import { Card } from 'primereact/card';
import { confirmDialog } from 'primereact/confirmdialog';
import { DataView, DataViewLayoutOptions } from 'primereact/dataview';
import { Divider } from 'primereact/divider';
import { Dropdown } from 'primereact/dropdown';
import { FileUpload, type FileUploadFile, type FileUploadHandlerEvent, type FileUploadSelectEvent, type FileUploadUploadEvent, type ItemTemplateOptions } from 'primereact/fileupload';
import { Message } from 'primereact/message';
import { ProgressBar } from 'primereact/progressbar';
import { Skeleton } from 'primereact/skeleton';
import { Tag } from 'primereact/tag';
import { type Toast } from 'primereact/toast';
import { Tooltip } from 'primereact/tooltip';
import { classNames } from 'primereact/utils';
import { format } from 'date-fns';

import { API_HOST, HttpError, apiFetch, apiUrl, getAccessToken } from '../api';
import { AppToast } from '../components/AppToast';
import { DocumentViewLayout } from '../components/DocumentViewLayout';
import { type DocumentFile } from '../models/documentFile';
import { useDocument } from '../queries/useDocuments';
import {
  useDocumentFiles,
  useDocumentFileOcrPages,
  useDocumentFilePageImage,
  useDocumentFilePages,
  useDocumentFileThumbnail,
  useDeleteDocumentFile,
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
  onDelete: (event: React.MouseEvent, file: DocumentFile) => void;
  isDownloading: boolean;
  isDeleting: boolean;
  canDelete: boolean;
};

type DocumentFileGridItemProps = {
  documentId: number;
  file: DocumentFile;
  onOpenPreview: (fileId: number) => void;
  onDownload: (event: React.MouseEvent, file: DocumentFile) => void;
  onDelete: (event: React.MouseEvent, file: DocumentFile) => void;
  isDownloading: boolean;
  isDeleting: boolean;
  canDelete: boolean;
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

function DeleteButton({ file, onDelete, isDeleting, canDelete }: Readonly<{
  file: DocumentFile;
  onDelete: (event: React.MouseEvent, file: DocumentFile) => void;
  isDeleting: boolean;
  canDelete: boolean;
}>) {
  return (
    <Button
      type="button"
      icon={isDeleting ? 'pi pi-spin pi-spinner' : 'pi pi-trash'}
      severity="danger"
      outlined
      size="small"
      className="aut-document-file-delete-button"
      aria-label={`Delete ${file.filename}`}
      onClick={(event) => onDelete(event, file)}
      disabled={isDeleting || !canDelete}
    />
  );
}

function DocumentFileActions({ file, onDownload, onDelete, isDownloading, isDeleting, canDelete }: Readonly<{
  file: DocumentFile;
  onDownload: (event: React.MouseEvent, file: DocumentFile) => void;
  onDelete: (event: React.MouseEvent, file: DocumentFile) => void;
  isDownloading: boolean;
  isDeleting: boolean;
  canDelete: boolean;
}>) {
  return (
    <div className="flex align-items-center gap-2 aut-document-file-actions">
      <DownloadButton file={file} onDownload={onDownload} isDownloading={isDownloading} />
      <DeleteButton file={file} onDelete={onDelete} isDeleting={isDeleting} canDelete={canDelete} />
    </div>
  );
}

function DocumentFileListItem({ documentId, file, index, onOpenPreview, onDownload, onDelete, isDownloading, isDeleting, canDelete }: Readonly<DocumentFileListItemProps>) {
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
              <DocumentFileActions file={file} onDownload={onDownload} onDelete={onDelete} isDownloading={isDownloading} isDeleting={isDeleting} canDelete={canDelete} />
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

function DocumentFileGridItem({ documentId, file, onOpenPreview, onDownload, onDelete, isDownloading, isDeleting, canDelete }: Readonly<DocumentFileGridItemProps>) {
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
              <DocumentFileActions file={file} onDownload={onDownload} onDelete={onDelete} isDownloading={isDownloading} isDeleting={isDeleting} canDelete={canDelete} />
            </div>
          </aside>
        </section>
      </div>
    </div>
  );
}

export function UploadDocumentFile() {
  const documentId = useId('id');
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const toast = useRef<Toast>(null);
  const [totalSize, setTotalSize] = useState(0);
  const fileUploadRef = useRef<FileUpload>(null);
  const [isUploading, setIsUploading] = useState(false);
  const [uploadProgress, setUploadProgress] = useState<number | null>(null);
  const [uploadStatus, setUploadStatus] = useState('');
  const { data: document, isLoading: isDocumentLoading, isError: isDocumentError, error: documentError } = useDocument(documentId);

  const onTemplateSelect = (event: FileUploadSelectEvent) => {
    const nextTotalSize = event.files.reduce((sum, file) => sum + (file.size || 0), 0);
    setTotalSize(nextTotalSize);
  };

  const onTemplateUpload = (event: FileUploadUploadEvent) => {
    const nextTotalSize = event.files.reduce((sum, file) => sum + (file.size || 0), 0);
    setTotalSize(nextTotalSize);
    toast.current?.show({ severity: 'info', summary: 'Success', detail: 'File Uploaded' });
  };

  const onTemplateRemove = (file: File, callback: (event: React.SyntheticEvent) => void, event: React.SyntheticEvent) => {
    setTotalSize((current) => Math.max(0, current - file.size));
    callback(event);
  };

  const onTemplateClear = () => {
    setTotalSize(0);
  };

  const headerTemplate = (options: { className: string; chooseButton: ReactNode; uploadButton: ReactNode; cancelButton: ReactNode }) => {
    const { className, chooseButton, uploadButton, cancelButton } = options;
    const formattedValue = fileUploadRef.current ? fileUploadRef.current.formatSize(totalSize) : '0 B';

    return (
      <div className={className} style={{ backgroundColor: 'transparent', display: 'flex', alignItems: 'center' }}>
        {chooseButton}
        {uploadButton}
        {cancelButton}
        <div className="flex align-items-center gap-3 ml-auto">
          <span>{formattedValue} selected</span>
        </div>
      </div>
    );
  };

  const itemTemplate = (file: FileUploadFile, props: ItemTemplateOptions) => {
    return (
      <div className="flex align-items-center flex-wrap">
        <div className="flex align-items-center" style={{ width: '40%' }}>
          {file.objectURL ? (
            <img alt={file.name} role="presentation" src={file.objectURL} width={100} />
          ) : (
            <span className="pi pi-file" style={{ fontSize: '3rem', width: '100px', textAlign: 'center' }} aria-hidden="true" />
          )}
          <span className="flex flex-column text-left ml-3">
            {file.name}
            <small>{new Date().toLocaleDateString()}</small>
          </span>
        </div>
        <Tag value={props.formatSize} severity="warning" className="px-3 py-2" />
        <Button
          type="button"
          icon="pi pi-times"
          className="p-button-outlined p-button-rounded p-button-danger ml-auto"
          onClick={(event) => onTemplateRemove(file, props.onRemove, event)}
        />
      </div>
    );
  };

  const uploadFile = (
    formData: FormData,
    file: File,
    onProgress: (loaded: number) => void,
    onUploadComplete: () => void,
  ) => {
    return new Promise<void>((resolve, reject) => {
      const xhr = new XMLHttpRequest();
      xhr.open('POST', apiUrl(`api/v1/documents/${documentId}/files`));
      xhr.withCredentials = true;
      const token = getAccessToken();
      if (token) xhr.setRequestHeader('Authorization', `Bearer ${token}`);

      xhr.upload.onprogress = (event) => {
        if (event.lengthComputable) {
          onProgress(event.loaded);
        }
      };
      xhr.upload.onload = onUploadComplete;

      xhr.onload = () => {
        if (xhr.status >= 200 && xhr.status < 300) {
          resolve();
          return;
        }

        let detail = `Upload failed (${xhr.status})`;
        try {
          const data = JSON.parse(xhr.responseText);
          if (typeof data?.message === 'string') detail = data.message;
        } catch {
          if (xhr.responseText) detail = xhr.responseText;
        }
        reject(new Error(detail));
      };

      xhr.onerror = () => reject(new Error(`Upload failed for ${file.name}`));
      xhr.onabort = () => reject(new Error(`Upload cancelled for ${file.name}`));
      xhr.send(formData);
    });
  };

  const uploadHandler = async (event: FileUploadHandlerEvent) => {
    const files = event.files ?? [];
    if (!files.length) return;

    setIsUploading(true);
    setUploadProgress(0);
    setUploadStatus(files.length === 1 ? `Uploading ${files[0].name}...` : `Uploading 1 of ${files.length}: ${files[0].name}`);

    const uploadedBytesByFile = new Map<File, number>();
    const totalBytes = files.reduce((sum, file) => sum + (file.size || 0), 0);

    const updateProgress = (file: File, loaded: number) => {
      uploadedBytesByFile.set(file, loaded);
      const loadedBytes = Array.from(uploadedBytesByFile.values()).reduce((sum, value) => sum + value, 0);
      setUploadProgress(totalBytes > 0 ? Math.min(100, Math.round((loadedBytes / totalBytes) * 100)) : null);
    };

    const uploadOne = async (file: File) => {
      const formData = new FormData();
      formData.append('file', file);

      await uploadFile(
        formData,
        file,
        (loaded) => updateProgress(file, loaded),
        () => {
          updateProgress(file, file.size || 0);
          setUploadStatus(`Processing ${file.name}...`);
        },
      );
      uploadedBytesByFile.set(file, file.size || 0);
    };

    const failures: unknown[] = [];
    for (const [index, file] of files.entries()) {
      setUploadStatus(files.length === 1 ? `Uploading ${file.name}...` : `Uploading ${index + 1} of ${files.length}: ${file.name}`);
      try {
        await uploadOne(file);
      } catch (error) {
        failures.push(error);
      }
    }

    if (failures.length) {
      const message = failures[0] instanceof Error
        ? failures[0].message
        : 'Some files failed to upload.';
      toast.current?.show({
        severity: 'error',
        summary: 'Upload incomplete',
        detail: message,
      });
      setIsUploading(false);
      setUploadProgress(null);
      setUploadStatus('');
      return;
    }

    event.options.clear();
    setTotalSize(0);
    setIsUploading(false);
    setUploadProgress(null);
    setUploadStatus('');
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: ['documentFile'] }),
      queryClient.invalidateQueries({ queryKey: ['document'] }),
      queryClient.invalidateQueries({ queryKey: ['documentFilePage'] }),
      queryClient.invalidateQueries({ queryKey: ['documentFilePageImage'] }),
    ]);
    const label = files.length === 1 ? 'File uploaded.' : `${files.length} files uploaded.`;
    toast.current?.show({ severity: 'success', summary: 'Success', detail: label });
  };

  const emptyTemplate = () => {
    return (
      <div className="flex align-items-center flex-column">
        <i className="pi pi-file mt-3 p-5" style={{ fontSize: '5em', borderRadius: '50%', backgroundColor: 'var(--surface-b)', color: 'var(--surface-d)' }}></i>
        <span style={{ fontSize: '1.2em', color: 'var(--text-color-secondary)' }} className="my-5">
          Drag and Drop File Here
        </span>
      </div>
    );
  };

  const chooseOptions = { label: 'Choose Files', icon: 'pi pi-fw pi-file', className: 'custom-choose-btn p-button-rounded p-button-outlined' };
  const uploadOptions = { label: 'Upload', icon: 'pi pi-fw pi-cloud-upload', className: 'custom-upload-btn p-button-success p-button-rounded p-button-outlined' };
  const cancelOptions = { label: 'Clear', icon: 'pi pi-fw pi-times', className: 'custom-cancel-btn p-button-danger p-button-rounded p-button-outlined' };

  if (isDocumentError) {
    return <Message severity="error" text={documentError.message} />;
  }

  if (isDocumentLoading) {
    return <div>Loading</div>;
  }

  return (
    <DocumentViewLayout documentId={documentId}>
      <Card title={`Add File${document?.title ? `: ${document.title}` : ''}`}>
        <AppToast ref={toast} />

        <div className="flex justify-content-end mb-3">
          <Button
            label="Back to Files"
            type="button"
            icon="pi pi-arrow-left"
            severity="secondary"
            outlined
            onClick={() => navigate(`/documents/${documentId}/files`)}
            disabled={isUploading}
          />
        </div>

        <Tooltip target=".custom-choose-btn" content="Choose" position="bottom" />
        <Tooltip target=".custom-upload-btn" content="Upload" position="bottom" />
        <Tooltip target=".custom-cancel-btn" content="Clear" position="bottom" />

        <FileUpload
          ref={fileUploadRef}
          name="file"
          customUpload
          uploadHandler={uploadHandler}
          multiple
          onUpload={onTemplateUpload}
          onSelect={onTemplateSelect}
          onError={onTemplateClear}
          onClear={onTemplateClear}
          headerTemplate={headerTemplate}
          emptyTemplate={emptyTemplate}
          itemTemplate={itemTemplate as (file: object, options: ItemTemplateOptions) => React.ReactNode}
          chooseOptions={chooseOptions}
          uploadOptions={uploadOptions}
          cancelOptions={cancelOptions}
          disabled={isUploading}
        />

        {isUploading && (
          <div className="aut-upload-progress-panel mt-4" role="status" aria-live="polite">
            <div className="aut-upload-progress-status">
              <span className="pi pi-spin pi-spinner" aria-hidden="true" />
              <span>{uploadStatus}</span>
              {uploadProgress !== null && <span className="aut-upload-progress-percent">{uploadProgress}%</span>}
            </div>
            {uploadProgress === null ? (
              <ProgressBar mode="indeterminate" showValue={false} className="aut-upload-progress-bar" />
            ) : (
              <ProgressBar value={uploadProgress} showValue={false} className="aut-upload-progress-bar" />
            )}
          </div>
        )}
      </Card>
    </DocumentViewLayout>
  );
}

export function ListDocumentFiles() {
  const documentId = useId('id');
  const navigate = useNavigate();
  const { data: document, isLoading: isDocumentLoading, isError: isDocumentError, error: documentError } = useDocument(documentId);
  const { data: files, isLoading: isFilesLoading, isError: isFilesError, error: filesError } = useDocumentFiles(documentId);
  const deleteDocumentFile = useDeleteDocumentFile();
  const [layout, setLayout] = useState<'list' | 'grid'>('grid');
  const [downloadingIds, setDownloadingIds] = useState<Set<number>>(new Set());
  const [downloadError, setDownloadError] = useState<string | null>(null);
  const [deleteError, setDeleteError] = useState<string | null>(null);
  const [deletingId, setDeletingId] = useState<number | null>(null);

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

  const deleteFile = async (file: DocumentFile) => {
    setDeleteError(null);
    setDeletingId(file.id);
    try {
      await deleteDocumentFile.mutateAsync({ documentId, fileId: file.id });
    } catch (error) {
      setDeleteError(error instanceof HttpError ? error.message : 'Failed to delete file');
    } finally {
      setDeletingId(null);
    }
  };

  const confirmDeleteFile = (event: React.MouseEvent, file: DocumentFile) => {
    event.preventDefault();
    event.stopPropagation();
    confirmDialog({
      message: `Are you sure you want to delete "${file.filename}"?`,
      header: 'Delete File',
      icon: 'pi pi-trash',
      defaultFocus: 'reject',
      acceptClassName: 'p-button-danger',
      accept: () => void deleteFile(file),
    });
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
          onDelete={confirmDeleteFile}
          isDownloading={downloadingIds.has(file.id)}
          isDeleting={deletingId === file.id}
          canDelete={(files?.length ?? 0) > 1}
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
        onDelete={confirmDeleteFile}
        isDownloading={downloadingIds.has(file.id)}
        isDeleting={deletingId === file.id}
        canDelete={(files?.length ?? 0) > 1}
      />
    );
  };

  const listTemplate = (items: DocumentFile[], currentLayout: 'list' | 'grid') => (
    <div className="grid grid-nogutter">{items.map((file, index) => itemTemplate(file, currentLayout, index))}</div>
  );

  const header = (
    <div className="flex flex-column gap-3 md:flex-row md:justify-content-between md:align-items-center">
      <Button
        label="Add a File"
        type="button"
        icon="pi pi-plus"
        size="small"
        onClick={() => navigate(`/documents/${documentId}/files/new`)}
      />
      <div className="flex justify-content-end">
        <DataViewLayoutOptions layout={layout} onChange={(event) => setLayout(event.value as 'list' | 'grid')} />
      </div>
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
        {deleteError && <Message severity="error" text={deleteError} />}
        <DataView
          value={files ?? []}
          loading={isFilesLoading}
          emptyMessage="No files available for this document."
          listTemplate={listTemplate}
          layout={layout}
          header={header}
        />
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
