import { useCallback, useMemo, useRef, useState } from 'react';
import { useForm, Controller } from 'react-hook-form';
import { Link, useNavigate, useParams } from 'react-router-dom';

import { Column } from 'primereact/column';
import { Card } from 'primereact/card';
import { Button } from 'primereact/button';
import { InputTextarea } from 'primereact/inputtextarea';
import { Checkbox } from 'primereact/checkbox';
import { classNames } from 'primereact/utils';

import { useDocumentIndexTemplate, useDocumentIndexTemplates, useDocumentIndexTemplateTree, useDeleteDocumentIndexTemplate, useSaveDocumentIndexTemplate } from '../queries/useDocumentIndexTemplates';
import { type DocumentIndexTemplate } from '../models/documentIndex';
import { Message } from 'primereact/message';
import { useId } from '../util';
import { TreeTable } from 'primereact/treetable';
import { Dropdown } from 'primereact/dropdown';
import type { TreeNode } from 'primereact/treenode';
import { confirmDialog, ConfirmDialog } from 'primereact/confirmdialog';
import { type Toast } from 'primereact/toast';
import { AppToast } from '../components/AppToast';

const MAX_DOCUMENT_INDEX_TEMPLATES = 200;

export function ListDocumentIndexTemplates() {
  const toast = useRef<Toast>(null);
  const { documentIndexId: documentIndexIdParam } = useParams();
  const documentIndexId = Number(documentIndexIdParam);
  const deleteTemplate = useDeleteDocumentIndexTemplate(documentIndexId);
  const navigate = useNavigate();
  const { isPending, data, isFetching } = useDocumentIndexTemplateTree(documentIndexId);
  const [expandedKeys, setExpandedKeys] = useState<Record<string, boolean>>({});

  const templateTemplate = (c: TreeNode) => {
    return (
      <Link className="title" to={`${c.data.id}/edit`}>{c.data.template}</Link>
    );
  }

  const flagTemplate = useCallback((value: boolean) => (
    value ?
      <i className="pi pi-check" style={{color: 'var(--green-600)'}} /> :
      <i className="pi pi-times" style={{color: 'var(--red-600)'}} />
  ), []);

  const isLeafTemplate = useCallback((c: TreeNode) => (
    flagTemplate(!!c.data.is_leaf)
  ), [flagTemplate]);

  const enabledTemplate = useCallback((c: TreeNode) => (
    flagTemplate(!!c.data.enabled)
  ), [flagTemplate]);

  const autoExpandedKeys = useMemo(() => {
    if (!data?.length) return {};

    const keys: Record<string, boolean> = {};
    const stack = [...data];

    while (stack.length) {
      const node = stack.pop();
      if (!node) continue;
      if (node.key) {
        keys[node.key] = true;
      }
      if (node.children?.length) {
        stack.push(...node.children);
      }
    }

    return keys;
  }, [data]);

  if (!documentIndexIdParam || Number.isNaN(documentIndexId))
    return <Message severity="error" text="Missing or invalid document index ID" />;

  const actionTemplate = (c: TreeNode) => {
    return (
      <div className="flex flex-wrap gap-2">
        <Button type="button" icon="pi pi-pencil" severity="success" rounded text raised aria-description="Edit"
          onClick={() => navigate(`${c.data.id}/edit`)}
        ></Button>
        <Button type="button" icon="pi pi-trash" severity="danger" rounded text raised aria-description="Delete"
          onClick={() => confirmDeleteTemplate(c.data)}
        ></Button>
      </div>
    );
  };

  const doDeleteTemplate = async (template: DocumentIndexTemplate) => {
    try {
      await deleteTemplate.mutateAsync(template.id);
      toast.current?.show({ severity: 'success', summary: 'Template deleted', detail: `Deleted ${template.template}.` });
    } catch (error) {
      const detail = error instanceof Error ? error.message : 'Something went wrong';
      toast.current?.show({ severity: 'error', summary: 'Delete failed', detail });
    }
  }

  const confirmDeleteTemplate = (template: DocumentIndexTemplate) => {
    confirmDialog({
      message: 'Are you sure want to delete this document index template?  All related index values will be deleted.',
      header: `Delete: ${template.template}`,
      icon: 'pi pi-trash',
      defaultFocus: 'reject',
      acceptClassName: 'p-button-danger',
      accept: () => void doDeleteTemplate(template),
    });
  };

  return (
    <>
    <Link to="new" style={{float: 'right', padding: '1.5rem'}}>New Document Index Template &raquo;</Link>
    <Card title="Document Index Templates">
      <TreeTable value={data}
          loading={isPending || isFetching}
          expandedKeys={Object.keys(expandedKeys).length ? expandedKeys : autoExpandedKeys}
          onToggle={(event) => setExpandedKeys(event.value)}
        >
        <Column field="template" header="Template" body={templateTemplate} sortable expander></Column>
        <Column field="is_leaf" header="Is Leaf" body={isLeafTemplate} sortable headerClassName="w-8rem"></Column>
        <Column field="enabled" header="Enabled" body={enabledTemplate} sortable headerClassName="w-8rem"></Column>
        <Column body={actionTemplate} headerClassName="w-9rem" />
      </TreeTable>
    </Card>
    <AppToast ref={toast} />
    <ConfirmDialog />
    </>
  );
}

export function EditDocumentIndexTemplate() {
  const id = useId('id');
  const { documentIndexId: documentIndexIdParam } = useParams();
  const documentIndexId = Number(documentIndexIdParam);
  const { isLoading, isError, data, error } = useDocumentIndexTemplate(documentIndexId, id);

  if (!documentIndexIdParam || Number.isNaN(documentIndexId))
    return <Message severity="error" text="Missing or invalid document index ID" />;
  if (!id)
    return <Message severity="error" text="Missing or invalid ID" />;
  if (isError)
    return <Message severity="error" text={error.message} />
  if (isLoading)
    return <div>Loading</div>;

  return (
    <Card title="Edit Document Index Template">
      { !isLoading && !isError && <DocumentIndexTemplateForm documentIndexId={documentIndexId} data={data} /> }
    </Card>
  );
}

export function NewDocumentIndexTemplate() {
  const { documentIndexId: documentIndexIdParam } = useParams();
  const documentIndexId = Number(documentIndexIdParam);

  if (!documentIndexIdParam || Number.isNaN(documentIndexId))
    return <Message severity="error" text="Missing or invalid document index ID" />;

  return (
    <Card title="New Document Index Template">
      <DocumentIndexTemplateForm documentIndexId={documentIndexId} />
    </Card>
  );
}

function DocumentIndexTemplateForm({
  documentIndexId,
  data,
}: Readonly<{ documentIndexId: number; data?: Partial<DocumentIndexTemplate> }>) {
  const saveTemplate = useSaveDocumentIndexTemplate(documentIndexId);
  const navigate = useNavigate();

  const {
    control,
    handleSubmit,
    formState: { errors, isSubmitting, isValid, isDirty },
  } = useForm<Partial<DocumentIndexTemplate>>({
    mode: 'onChange', // validate as user types
    defaultValues: {
      enabled: true,
      is_leaf: false,
    },
    values: data ?? {},
  });

  const submitter = async (data: Partial<DocumentIndexTemplate>) => {
    await saveTemplate.mutateAsync(data, {
      onSuccess: () => {
        navigate(`/indexes/${documentIndexId}/templates`);
      }
    });
  };

  // PrimeReact-friendly error helper
  const errMsg = (name: keyof Partial<DocumentIndexTemplate>) =>
    errors[name]?.message ? String(errors[name]?.message) : null;

  const {
    data: parents,
    isLoading: isParentsLoading,
    isError: isParentsError,
  } = useDocumentIndexTemplates(documentIndexId, {page:1, per_page: MAX_DOCUMENT_INDEX_TEMPLATES});
  let parent_id_options: DocumentIndexTemplate[] = [];
  if (!isParentsLoading && !isParentsError && parents?.items?.length) {
    parent_id_options = parents.items.filter(c => c.id !== data?.id);
  }

  return (
    <form onSubmit={handleSubmit(submitter)}>
      <div className="grid p-fluid">

        {/* Template */}
        <div className="col-12">
          <label htmlFor="template" className="font-medium mb-2 block">Template</label>
          <Controller name="template" control={control}
            rules={{
              required: 'Template is required',
              minLength: { value: 2, message: 'Template must be at least 2 characters' },
            }}
            render={({ field }) => (
              <InputTextarea id="template" {...field}
                className={classNames({ 'p-invalid': !!errors.template })}
                placeholder="Template" autoComplete="template"
                rows={4}
              />
            )}
          />
          {errMsg('template') && <small className="p-error">{errMsg('template')}</small>}
        </div>

        {/* Parent */}
        <div className="col-12 md:col-6 lg:col-4">
          <label htmlFor="parent_id" className="font-medium mb-2 block">Parent Template</label>
          <Controller name="parent_id" control={control}
            render={({ field }) => (
              <Dropdown id="parent_id" {...field}
                optionLabel="template" optionValue="id"
                className={classNames({ 'p-invalid': !!errors.parent_id })}
                placeholder="Parent template" options={parent_id_options} autoComplete="parent_id"
              />
            )}
          />
          {errMsg('parent_id') && <small className="p-error">{errMsg('parent_id')}</small>}
        </div>

        {/* Is Leaf */}
        <div className="col-12 md:col-6 lg:col-4">
          <label htmlFor="is_leaf" className="font-medium mb-2 block">&nbsp;</label>
          <Controller name="is_leaf" control={control}
            render={({ field }) => (
              <div className="flex align-items-center gap-2">
                <Checkbox
                  inputId="is_leaf"
                  checked={field.value ?? false}
                  onChange={(event) => field.onChange(event.checked ?? false)}
                />
                <label htmlFor="is_leaf">Is Leaf</label>
              </div>
            )}
          />
          {errMsg('is_leaf') && <small className="p-error">{errMsg('is_leaf')}</small>}
        </div>

        {/* Enabled */}
        <div className="col-12 md:col-6 lg:col-4">
          <label htmlFor="enabled" className="font-medium mb-2 block">&nbsp;</label>
          <Controller name="enabled" control={control}
            render={({ field }) => (
              <div className="flex align-items-center gap-2">
                <Checkbox
                  inputId="enabled"
                  checked={field.value ?? false}
                  onChange={(event) => field.onChange(event.checked ?? false)}
                />
                <label htmlFor="enabled">Enabled</label>
              </div>
            )}
          />
          {errMsg('enabled') && <small className="p-error">{errMsg('enabled')}</small>}
        </div>
      </div>

      <div className="text-end">
        {saveTemplate.isError && (
          <Message className="float-start" severity="error" text={saveTemplate.error.message} />
        )}

        <Button label="Save" type="submit" icon="pi pi-check" raised disabled={!isDirty || !isValid || isSubmitting} />
        <Button label="Cancel" type="button" severity="secondary" icon="pi pi-times" raised onClick={() => navigate(`/indexes/${documentIndexId}/templates`)} />
      </div>
    </form>
  );
}
