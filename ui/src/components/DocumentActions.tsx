import { useId, useRef, useState, type CSSProperties } from 'react';

import { Button } from 'primereact/button';
import { Dialog } from 'primereact/dialog';
import { Dropdown } from 'primereact/dropdown';
import { Menu } from 'primereact/menu';
import type { MenuItem } from 'primereact/menuitem';
import { PickList, type PickListChangeEvent } from 'primereact/picklist';
import { TreeSelect } from 'primereact/treeselect';
import { confirmDialog, ConfirmDialog } from 'primereact/confirmdialog';
import { type Toast } from 'primereact/toast';

import { AppToast } from './AppToast';
import { useCabinets, useCabinetTree } from '../queries/useCabinets';
import { useTags } from '../queries/useTags';
import { downloadDocumentListCsv, useClassifyDocument, useDeleteDocument, useGenerateThumbnail, useProcessDocumentFilePages, useRemoveCabinetDocument, useRemoveTagDocument, useSaveCabinetDocument, useSaveTagDocument } from '../queries/useDocuments';
import { downloadFirstDocumentFile } from '../queries/useDocumentFiles';
import type { Document, DocumentListParams } from '../models/document';
import { MAX_CABINETS } from '../models/cabinet';
import { useMetadataTypes } from '../queries/useMetadataTypes';
import type { MetadataType } from '../models/metadataType';

type DocumentActionsProps = {
  documentIds: number[];
  documents?: Document[];
  onAfterAction?: () => void;
  onAfterDelete?: () => void;
  includeNewDocument?: boolean;
  csvExportParams?: DocumentListParams;
  buttonClassName?: string;
  buttonStyle?: CSSProperties;
  containerClassName?: string;
};

export function DocumentActions({
  documentIds,
  documents,
  onAfterAction,
  onAfterDelete,
  includeNewDocument = false,
  csvExportParams,
  buttonClassName,
  buttonStyle,
  containerClassName,
}: Readonly<DocumentActionsProps>) {
  const actionMenu = useRef<Menu>(null);
  const toast = useRef<Toast>(null);
  const menuId = useId();
  const hasSelection = documentIds.length > 0;
  const hasCsvExport = !!csvExportParams;
  const deleteDocument = useDeleteDocument();
  const processDocumentFilePages = useProcessDocumentFilePages();
  const generateThumbnail = useGenerateThumbnail();
  const classifyDocument = useClassifyDocument();
  const saveCabinetDocument = useSaveCabinetDocument();
  const removeCabinetDocument = useRemoveCabinetDocument();
  const saveTagDocument = useSaveTagDocument();
  const removeTagDocument = useRemoveTagDocument();
  const { data: cabinetTreeOptions, isPending: isCabinetTreePending, isFetching: isCabinetTreeFetching } = useCabinetTree({ keyField: 'id' });
  const { data: cabinetOptions, isPending: isCabinetsPending, isFetching: isCabinetsFetching } = useCabinets({ page: 1, per_page: MAX_CABINETS, sf: 'name' });
  const { data: tagOptions, isPending: isTagsPending, isFetching: isTagsFetching } = useTags({ page: 1, per_page: 200, sf: 'name' });
  const { data: metadataTypes, isPending: isMetadataTypesPending, isFetching: isMetadataTypesFetching } = useMetadataTypes({ page: 1, per_page: 200, sf: 'name' });
  const selectedDocumentCabinetIds = new Set(documents?.flatMap((document) => document.cabinet_ids ?? []) ?? []);
  const removeCabinetOptions = (cabinetOptions?.items ?? [])
    .filter((cabinet) => selectedDocumentCabinetIds.has(cabinet.id))
    .sort((left, right) => (left.displayName ?? left.name ?? left.slug).localeCompare(right.displayName ?? right.name ?? right.slug));
  const addTagOptions = tagOptions?.items ?? [];
  const selectedDocumentTagIds = new Set(documents?.flatMap((document) => document.tag_ids ?? []) ?? []);
  const removeTagOptions = addTagOptions.filter((tag) => selectedDocumentTagIds.has(tag.id));
  const [addToCabinetVisible, setAddToCabinetVisible] = useState(false);
  const [selectedCabinetId, setSelectedCabinetId] = useState<number | null>(null);
  const [removeFromCabinetVisible, setRemoveFromCabinetVisible] = useState(false);
  const [removeCabinetId, setRemoveCabinetId] = useState<number | null>(null);
  const [addTagVisible, setAddTagVisible] = useState(false);
  const [selectedTagId, setSelectedTagId] = useState<number | null>(null);
  const [removeTagVisible, setRemoveTagVisible] = useState(false);
  const [removeTagId, setRemoveTagId] = useState<number | null>(null);
  const [isDownloading, setIsDownloading] = useState(false);
  const [csvExportVisible, setCsvExportVisible] = useState(false);
  const [selectedCsvMetadataTypes, setSelectedCsvMetadataTypes] = useState<MetadataType[]>([]);
  const [isCsvExporting, setIsCsvExporting] = useState(false);

  const csvMetadataSource = (metadataTypes?.items ?? []).filter(
    (metadataType) => !selectedCsvMetadataTypes.some((selected) => selected.id === metadataType.id),
  );

  const showSuccess = (summary: string, detail: string) => {
    toast.current?.show({ severity: 'success', summary, detail });
  };

  const showError = (summary: string, error: unknown) => {
    const detail = error instanceof Error ? error.message : 'Something went wrong';
    toast.current?.show({ severity: 'error', summary, detail });
  };

  const documentCountLabel = (count: number, singular: string, plural: string = `${singular}s`) => (
    `${count} ${count === 1 ? singular : plural}`
  );

  const openAddToCabinetDialog = () => {
    if (!hasSelection) return;
    setAddToCabinetVisible(true);
  };

  const closeAddToCabinetDialog = () => {
    setAddToCabinetVisible(false);
    setSelectedCabinetId(null);
  };

  const saveAddToCabinet = async () => {
    const currentDocumentIds = documentIds;
    const currentSelectionCount = currentDocumentIds.length;
    if (!selectedCabinetId || currentSelectionCount === 0) return;
    try {
      const documents = currentDocumentIds.map((id) => ({ document_id: id }));
      await saveCabinetDocument.mutateAsync({ cabinet_id: selectedCabinetId, documents });
      closeAddToCabinetDialog();
      onAfterAction?.();
      showSuccess('Cabinet updated', `Added ${documentCountLabel(currentSelectionCount, 'document')} to cabinet.`);
    } catch (error) {
      showError('Add to cabinet failed', error);
    }
  };

  const openRemoveFromCabinetDialog = () => {
    if (!hasSelection) return;
    setRemoveFromCabinetVisible(true);
  };

  const closeRemoveFromCabinetDialog = () => {
    setRemoveFromCabinetVisible(false);
    setRemoveCabinetId(null);
  };

  const saveRemoveFromCabinet = async () => {
    const currentDocumentIds = documentIds;
    const currentSelectionCount = currentDocumentIds.length;
    if (!removeCabinetId || currentSelectionCount === 0) return;
    try {
      await removeCabinetDocument.mutateAsync({ cabinet_id: removeCabinetId, documents: currentDocumentIds });
      closeRemoveFromCabinetDialog();
      onAfterAction?.();
      showSuccess('Cabinet updated', `Removed ${documentCountLabel(currentSelectionCount, 'document')} from cabinet.`);
    } catch (error) {
      showError('Remove from cabinet failed', error);
    }
  };

  const openAddTagDialog = () => {
    if (!hasSelection) return;
    setAddTagVisible(true);
  };

  const closeAddTagDialog = () => {
    setAddTagVisible(false);
    setSelectedTagId(null);
  };

  const saveAddTag = async () => {
    const currentDocumentIds = documentIds;
    const currentSelectionCount = currentDocumentIds.length;
    if (!selectedTagId || currentSelectionCount === 0) return;
    try {
      const documents = currentDocumentIds.map((id) => ({ document_id: id }));
      await saveTagDocument.mutateAsync({ tag_id: selectedTagId, documents });
      closeAddTagDialog();
      onAfterAction?.();
      showSuccess('Tags updated', `Added tag to ${documentCountLabel(currentSelectionCount, 'document')}.`);
    } catch (error) {
      showError('Add tag failed', error);
    }
  };

  const openRemoveTagDialog = () => {
    if (!hasSelection) return;
    setRemoveTagVisible(true);
  };

  const closeRemoveTagDialog = () => {
    setRemoveTagVisible(false);
    setRemoveTagId(null);
  };

  const saveRemoveTag = async () => {
    const currentDocumentIds = documentIds;
    const currentSelectionCount = currentDocumentIds.length;
    if (!removeTagId || currentSelectionCount === 0) return;
    try {
      await removeTagDocument.mutateAsync({ tag_id: removeTagId, documents: currentDocumentIds });
      closeRemoveTagDialog();
      onAfterAction?.();
      showSuccess('Tags updated', `Removed tag from ${documentCountLabel(currentSelectionCount, 'document')}.`);
    } catch (error) {
      showError('Remove tag failed', error);
    }
  };

  const deleteSelectedDocuments = async () => {
    const currentDocumentIds = documentIds;
    const currentSelectionCount = currentDocumentIds.length;
    if (currentSelectionCount === 0) return;
    try {
      await Promise.all(currentDocumentIds.map((id) => deleteDocument.mutateAsync(id)));
      onAfterAction?.();
      onAfterDelete?.();
      showSuccess('Documents deleted', `Deleted ${documentCountLabel(currentSelectionCount, 'document')}.`);
    } catch (error) {
      showError('Delete failed', error);
    }
  };

  const reprocessSelectedDocuments = async () => {
    const currentDocumentIds = documentIds;
    const currentSelectionCount = currentDocumentIds.length;
    if (currentSelectionCount === 0) return;
    try {
      await Promise.all(currentDocumentIds.map((id) => processDocumentFilePages.mutateAsync(id)));
      onAfterAction?.();
      showSuccess('Reprocess queued', `Queued page processing for ${documentCountLabel(currentSelectionCount, 'document')}.`);
    } catch (error) {
      showError('Reprocess failed', error);
    }
  };

  const generateThumbnailsForSelectedDocuments = async () => {
    const currentDocumentIds = documentIds;
    const currentSelectionCount = currentDocumentIds.length;
    if (currentSelectionCount === 0) return;
    try {
      await Promise.all(currentDocumentIds.map((id) => generateThumbnail.mutateAsync(id)));
      onAfterAction?.();
      showSuccess('Thumbnail queued', `Queued thumbnail generation for ${documentCountLabel(currentSelectionCount, 'document')}.`);
    } catch (error) {
      showError('Generate thumbnail failed', error);
    }
  };

  const classifySelectedDocuments = async () => {
    const currentDocumentIds = documentIds;
    const currentSelectionCount = currentDocumentIds.length;
    if (currentSelectionCount === 0) return;
    try {
      await Promise.all(currentDocumentIds.map((id) => classifyDocument.mutateAsync(id)));
      onAfterAction?.();
      showSuccess('Classification queued', `Queued classification for ${documentCountLabel(currentSelectionCount, 'document')}.`);
    } catch (error) {
      showError('Classification failed', error);
    }
  };

  const downloadSelectedDocuments = async () => {
    const currentDocumentIds = [...documentIds];
    if (currentDocumentIds.length === 0 || isDownloading) return;
    setIsDownloading(true);
    try {
      let downloadedCount = 0;
      let failedCount = 0;
      for (const [index, id] of currentDocumentIds.entries()) {
        // Stagger downloads so the browser has a chance to start each one
        // before the next anchor click fires.
        if (index > 0) {
          await new Promise((resolve) => window.setTimeout(resolve, 500));
        }
        try {
          const downloaded = await downloadFirstDocumentFile(id);
          if (downloaded) downloadedCount += 1;
        } catch (error) {
          failedCount += 1;
          showError(`Download failed (document ${id})`, error);
        }
      }
      if (downloadedCount > 0) {
        const detail = currentDocumentIds.length > 1
          ? `Downloading first file from ${documentCountLabel(downloadedCount, 'document')}. If your browser blocked additional files, allow multiple automatic downloads for this site and try again.`
          : `Downloading first file from ${documentCountLabel(downloadedCount, 'document')}.`;
        showSuccess('Download started', detail);
      } else if (failedCount === 0) {
        showSuccess('Nothing to download', 'The selected documents have no files.');
      }
    } finally {
      setIsDownloading(false);
    }
  };

  const openCsvExportDialog = () => {
    if (!hasCsvExport) return;
    setCsvExportVisible(true);
  };

  const closeCsvExportDialog = () => {
    setCsvExportVisible(false);
  };

  const downloadCsvExport = async () => {
    if (!csvExportParams || isCsvExporting) return;
    setIsCsvExporting(true);
    try {
      await downloadDocumentListCsv(
        csvExportParams,
        selectedCsvMetadataTypes.map((metadataType) => metadataType.id),
      );
      closeCsvExportDialog();
      showSuccess('CSV download started', 'Exporting all matching documents.');
    } catch (error) {
      showError('CSV export failed', error);
    } finally {
      setIsCsvExporting(false);
    }
  };

  const metadataTypeTemplate = (metadataType: MetadataType) => (
    <div className="flex flex-column">
      <span>{metadataType.name}</span>
      {metadataType.description && <small className="text-color-secondary">{metadataType.description}</small>}
    </div>
  );

  const confirmDeleteSelectedDocuments = () => {
    if (!hasSelection) return;
    const count = documentIds.length;
    const label = count === 1 ? 'document' : 'documents';
    confirmDialog({
      message: `Are you sure you want to delete ${count} ${label}?`,
      header: count === 1 ? 'Delete Document' : 'Delete Documents',
      icon: 'pi pi-trash',
      defaultFocus: 'reject',
      acceptClassName: 'p-button-danger',
      accept: () => void deleteSelectedDocuments(),
    });
  };

  const confirmReprocessSelectedDocuments = () => {
    if (!hasSelection) return;
    const count = documentIds.length;
    const message = count === 1
      ? 'Are you sure you want to reprocess pages for this document?'
      : 'Are you sure you want to reprocess pages for these documents?';

    confirmDialog({
      message,
      header: 'Reprocess Pages',
      icon: 'pi pi-refresh',
      defaultFocus: 'reject',
      accept: () => void reprocessSelectedDocuments(),
    });
  };

  const confirmGenerateThumbnailsForSelectedDocuments = () => {
    if (!hasSelection) return;
    const count = documentIds.length;
    const message = count === 1
      ? 'Are you sure you want to generate a thumbnail for this document?'
      : 'Are you sure you want to generate thumbnails for these documents?';

    confirmDialog({
      message,
      header: 'Generate Thumbnail',
      icon: 'pi pi-image',
      defaultFocus: 'reject',
      accept: () => void generateThumbnailsForSelectedDocuments(),
    });
  };

  const confirmClassifySelectedDocuments = () => {
    if (!hasSelection) return;
    const count = documentIds.length;
    const message = count === 1
      ? 'Are you sure you want to classify this document?'
      : 'Are you sure you want to classify these documents?';

    confirmDialog({
      message,
      header: 'Classify Document',
      icon: 'pi pi-bolt',
      defaultFocus: 'reject',
      accept: () => void classifySelectedDocuments(),
    });
  };

  const actionMenuItems: MenuItem[] = [
    ...(includeNewDocument
      ? [
          { icon: 'pi pi-upload', label: 'New Document', url: '/documents/new' },
        ]
      : []),
    { icon: 'pi pi-download', label: 'Download', command: () => { void downloadSelectedDocuments(); }, disabled: !hasSelection || isDownloading },
    ...(hasCsvExport
      ? [
          { icon: 'pi pi-file', label: 'Export CSV', command: () => { openCsvExportDialog(); }, disabled: isCsvExporting },
        ]
      : []),
    { separator: true },
    { icon: 'pi pi-plus-circle', label: 'Add to Cabinet', command: () => { openAddToCabinetDialog(); }, disabled: !hasSelection },
    { icon: 'pi pi-minus-circle', label: 'Remove from Cabinet', command: () => { openRemoveFromCabinetDialog(); }, disabled: !hasSelection },
    { separator: true },
    { icon: 'pi pi-plus-circle', label: 'Add Tag', command: () => { openAddTagDialog(); }, disabled: !hasSelection },
    { icon: 'pi pi-minus-circle', label: 'Remove Tag', command: () => { openRemoveTagDialog(); }, disabled: !hasSelection },
    { separator: true },
    { icon: 'pi pi-image', label: 'Generate Thumbnail', command: () => { confirmGenerateThumbnailsForSelectedDocuments(); }, disabled: !hasSelection || generateThumbnail.isPending },
    { icon: 'pi pi-refresh', label: 'Reprocess Pages', command: () => { confirmReprocessSelectedDocuments(); }, disabled: !hasSelection || processDocumentFilePages.isPending },
    { icon: 'pi pi-bolt', label: 'Classify Document', command: () => { confirmClassifySelectedDocuments(); }, disabled: !hasSelection || classifyDocument.isPending },
    { separator: true },
    { icon: 'pi pi-trash', label: 'Delete Document', command: () => { confirmDeleteSelectedDocuments(); }, disabled: !hasSelection },
  ];

  return (
    <div className={containerClassName}>
      <Menu model={actionMenuItems} popup ref={actionMenu} popupAlignment="right" id={menuId} style={{ minWidth: '16rem' }} />
      <Button
        label="Actions"
        className={buttonClassName}
        style={buttonStyle}
        size="small"
        raised
        onClick={(event) => actionMenu.current?.toggle(event)}
        aria-controls={menuId}
        aria-haspopup
      />

      <Dialog
        header="Export Document List to CSV"
        visible={csvExportVisible}
        onHide={closeCsvExportDialog}
        style={{ width: '95vw', maxWidth: '900px' }}
        dismissableMask={true}
        footer={(
          <div className="flex justify-content-end gap-2">
            <Button label="Cancel" type="button" severity="secondary" icon="pi pi-times" onClick={closeCsvExportDialog} />
            <Button
              label="Download CSV"
              type="button"
              icon={isCsvExporting ? 'pi pi-spin pi-spinner' : 'pi pi-download'}
              onClick={() => void downloadCsvExport()}
              disabled={isCsvExporting}
            />
          </div>
        )}
      >
        <p className="mt-0 text-color-secondary">
          Each row includes document ID, title, and preview URL.
        </p>
        <p className="mt-0 text-color-secondary">
          Choose metadata fields to add after those columns.
        </p>
        <PickList
          dataKey="id"
          source={csvMetadataSource}
          target={selectedCsvMetadataTypes}
          onChange={(event: PickListChangeEvent) => setSelectedCsvMetadataTypes(event.target as MetadataType[])}
          itemTemplate={metadataTypeTemplate}
          sourceHeader="Available Metadata"
          targetHeader="Selected Metadata"
          sourceStyle={{ height: '18rem' }}
          targetStyle={{ height: '18rem' }}
          breakpoint="900px"
          filter
          filterBy="name"
          showSourceControls={false}
          sourceFilterPlaceholder="Search available"
          targetFilterPlaceholder="Search selected"
        />
        {(isMetadataTypesPending || isMetadataTypesFetching) && (
          <small className="text-color-secondary block mt-2">Loading metadata fields...</small>
        )}
      </Dialog>
      <Dialog
        header="Add to Cabinet"
        visible={addToCabinetVisible}
        onHide={closeAddToCabinetDialog}
        style={{ width: '90vw', maxWidth: '520px' }}
        dismissableMask={true}
        footer={(
          <div className="flex justify-content-end gap-2">
            <Button label="Cancel" type="button" severity="secondary" icon="pi pi-times" onClick={closeAddToCabinetDialog} />
            <Button
              label="Save"
              type="button"
              icon="pi pi-check"
              onClick={() => void saveAddToCabinet()}
              disabled={!selectedCabinetId || !hasSelection || saveCabinetDocument.isPending}
            />
          </div>
        )}
      >
        <div className="grid p-fluid">
          <div className="col-12">
            <label htmlFor="cabinet_id" className="font-medium mb-2 block">Cabinet</label>
            <TreeSelect
              inputId="cabinet_id"
              value={selectedCabinetId ? String(selectedCabinetId) : null}
              onChange={(event) => setSelectedCabinetId(event.value ? Number(event.value) : null)}
              placeholder="Select a cabinet"
              options={cabinetTreeOptions ?? []}
              disabled={isCabinetTreePending || isCabinetTreeFetching}
              filter
              className="w-full"
            />
          </div>
        </div>
      </Dialog>
      <Dialog
        header="Remove from Cabinet"
        visible={removeFromCabinetVisible}
        onHide={closeRemoveFromCabinetDialog}
        style={{ width: '90vw', maxWidth: '520px' }}
        dismissableMask={true}
        footer={(
          <div className="flex justify-content-end gap-2">
            <Button label="Cancel" type="button" severity="secondary" icon="pi pi-times" onClick={closeRemoveFromCabinetDialog} />
            <Button
              label="Remove"
              type="button"
              severity="danger"
              icon="pi pi-minus-circle"
              onClick={() => void saveRemoveFromCabinet()}
              disabled={!removeCabinetId || !hasSelection || removeCabinetDocument.isPending}
            />
          </div>
        )}
      >
        <div className="grid p-fluid">
          <div className="col-12">
            <label htmlFor="remove_cabinet_id" className="font-medium mb-2 block">Cabinet</label>
            <Dropdown
              id="remove_cabinet_id"
              value={removeCabinetId}
              onChange={(event) => setRemoveCabinetId(event.value as number)}
              optionLabel="displayName"
              optionValue="id"
              placeholder={removeCabinetOptions.length > 0 ? 'Select a cabinet' : 'No cabinets on selected documents'}
              options={removeCabinetOptions}
              loading={isCabinetsPending || isCabinetsFetching}
              className="w-full"
            />
          </div>
        </div>
      </Dialog>
      <Dialog
        header="Add Tag"
        visible={addTagVisible}
        onHide={closeAddTagDialog}
        style={{ width: '90vw', maxWidth: '520px' }}
        dismissableMask={true}
        footer={(
          <div className="flex justify-content-end gap-2">
            <Button label="Cancel" type="button" severity="secondary" icon="pi pi-times" onClick={closeAddTagDialog} />
            <Button
              label="Save"
              type="button"
              icon="pi pi-check"
              onClick={() => void saveAddTag()}
              disabled={!selectedTagId || !hasSelection || saveTagDocument.isPending}
            />
          </div>
        )}
      >
        <div className="grid p-fluid">
          <div className="col-12">
            <label htmlFor="tag_id" className="font-medium mb-2 block">Tag</label>
            <Dropdown
              id="tag_id"
              value={selectedTagId}
              onChange={(event) => setSelectedTagId(event.value as number)}
              optionLabel="name"
              optionValue="id"
              placeholder="Select a tag"
              options={addTagOptions}
              loading={isTagsPending || isTagsFetching}
              className="w-full"
            />
          </div>
        </div>
      </Dialog>
      <Dialog
        header="Remove Tag"
        visible={removeTagVisible}
        onHide={closeRemoveTagDialog}
        style={{ width: '90vw', maxWidth: '520px' }}
        dismissableMask={true}
        footer={(
          <div className="flex justify-content-end gap-2">
            <Button label="Cancel" type="button" severity="secondary" icon="pi pi-times" onClick={closeRemoveTagDialog} />
            <Button
              label="Remove"
              type="button"
              severity="danger"
              icon="pi pi-minus-circle"
              onClick={() => void saveRemoveTag()}
              disabled={!removeTagId || !hasSelection || removeTagDocument.isPending}
            />
          </div>
        )}
      >
        <div className="grid p-fluid">
          <div className="col-12">
            <label htmlFor="remove_tag_id" className="font-medium mb-2 block">Tag</label>
            <Dropdown
              id="remove_tag_id"
              value={removeTagId}
              onChange={(event) => setRemoveTagId(event.value as number)}
              optionLabel="name"
              optionValue="id"
              placeholder={removeTagOptions.length > 0 ? 'Select a tag' : 'No tags on selected documents'}
              options={removeTagOptions}
              loading={isTagsPending || isTagsFetching}
              className="w-full"
            />
          </div>
        </div>
      </Dialog>
      <ConfirmDialog />
      <AppToast ref={toast} />
    </div>
  );
}
