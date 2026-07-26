import { useEffect, useId, useRef, useState, type CSSProperties } from 'react';

import { Button } from 'primereact/button';
import { Dialog } from 'primereact/dialog';
import { Dropdown } from 'primereact/dropdown';
import { Menu } from 'primereact/menu';
import type { MenuItem } from 'primereact/menuitem';
import { TreeSelect } from 'primereact/treeselect';
import { confirmDialog, ConfirmDialog } from 'primereact/confirmdialog';
import { type Toast } from 'primereact/toast';

import { AppToast } from './AppToast';
import { useCabinetTree } from '../queries/useCabinets';
import { useTags } from '../queries/useTags';
import { useClassifyDocument, useDeleteDocument, useGenerateThumbnail, useProcessDocumentFilePages, useRemoveCabinetDocument, useRemoveTagDocument, useSaveCabinetDocument, useSaveTagDocument } from '../queries/useDocuments';

type DocumentActionsProps = {
  documentIds: number[];
  onAfterAction?: () => void;
  onAfterDelete?: () => void;
  includeNewDocument?: boolean;
  buttonClassName?: string;
  buttonStyle?: CSSProperties;
  containerClassName?: string;
};

export function DocumentActions({
  documentIds,
  onAfterAction,
  onAfterDelete,
  includeNewDocument = false,
  buttonClassName,
  buttonStyle,
  containerClassName,
}: Readonly<DocumentActionsProps>) {
  const actionMenu = useRef<Menu>(null);
  const toast = useRef<Toast>(null);
  const documentIdsRef = useRef(documentIds);
  const menuId = useId();
  const hasSelection = documentIds.length > 0;
  const deleteDocument = useDeleteDocument();
  const processDocumentFilePages = useProcessDocumentFilePages();
  const generateThumbnail = useGenerateThumbnail();
  const classifyDocument = useClassifyDocument();
  const saveCabinetDocument = useSaveCabinetDocument();
  const removeCabinetDocument = useRemoveCabinetDocument();
  const saveTagDocument = useSaveTagDocument();
  const removeTagDocument = useRemoveTagDocument();
  const { data: cabinetOptions, isPending: isCabinetsPending, isFetching: isCabinetsFetching } = useCabinetTree({ keyField: 'id' });
  const { data: tagOptions, isPending: isTagsPending, isFetching: isTagsFetching } = useTags({ page: 1, per_page: 200, sf: 'name' });
  const [addToCabinetVisible, setAddToCabinetVisible] = useState(false);
  const [selectedCabinetId, setSelectedCabinetId] = useState<number | null>(null);
  const [removeFromCabinetVisible, setRemoveFromCabinetVisible] = useState(false);
  const [removeCabinetId, setRemoveCabinetId] = useState<number | null>(null);
  const [addTagVisible, setAddTagVisible] = useState(false);
  const [selectedTagId, setSelectedTagId] = useState<number | null>(null);
  const [removeTagVisible, setRemoveTagVisible] = useState(false);
  const [removeTagId, setRemoveTagId] = useState<number | null>(null);

  useEffect(() => {
    documentIdsRef.current = documentIds;
  }, [documentIds]);

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
    const currentDocumentIds = documentIdsRef.current;
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
    const currentDocumentIds = documentIdsRef.current;
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
    const currentDocumentIds = documentIdsRef.current;
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
    const currentDocumentIds = documentIdsRef.current;
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
    const currentDocumentIds = documentIdsRef.current;
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
    const currentDocumentIds = documentIdsRef.current;
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
    const currentDocumentIds = documentIdsRef.current;
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
    const currentDocumentIds = documentIdsRef.current;
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

  const actionMenuItems: MenuItem[] = [];
  if (includeNewDocument) {
    actionMenuItems.push(
      { icon: 'pi pi-upload', label: 'New Document', url: '/documents/new' },
      { separator: true },
    );
  }
  actionMenuItems.push(
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
  );

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
              options={cabinetOptions ?? []}
              disabled={isCabinetsPending || isCabinetsFetching}
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
            <TreeSelect
              inputId="remove_cabinet_id"
              value={removeCabinetId ? String(removeCabinetId) : null}
              onChange={(event) => setRemoveCabinetId(event.value ? Number(event.value) : null)}
              placeholder="Select a cabinet"
              options={cabinetOptions ?? []}
              disabled={isCabinetsPending || isCabinetsFetching}
              filter
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
              options={tagOptions?.items ?? []}
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
              placeholder="Select a tag"
              options={tagOptions?.items ?? []}
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
