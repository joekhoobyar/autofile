import { useEffect, useMemo, useRef, useState } from 'react';
import { Controller, useForm, useWatch } from 'react-hook-form';
import { Link, useNavigate } from 'react-router-dom';

import { DataTable, type DataTableStateEvent } from 'primereact/datatable';
import { Column } from 'primereact/column';
import { Card } from 'primereact/card';
import { Button } from 'primereact/button';
import { InputText } from 'primereact/inputtext';
import { Checkbox } from 'primereact/checkbox';
import { Message } from 'primereact/message';
import { Toast } from 'primereact/toast';
import { ConfirmDialog, confirmDialog } from 'primereact/confirmdialog';
import { classNames } from 'primereact/utils';
import CodeMirror from '@uiw/react-codemirror';
import { yaml as yamlLanguage } from '@codemirror/lang-yaml';
import { indentWithTab } from '@codemirror/commands';
import { EditorView, keymap } from '@codemirror/view';
import { vscodeDark } from '@uiw/codemirror-theme-vscode';

import type { ListParams } from '../api';
import { type ClassifierBlock } from '../models/classifierBlock';
import { useClassifierBlock, useClassifierBlocks, useDeleteClassifierBlock, useSaveClassifierBlock } from '../queries/useClassifierBlocks';
import { useId } from '../util';
import { defaultClassifierRules, rulesToYaml, yamlToRules } from '../util/classifierRulesYaml';

const yamlEditorExtensions = [yamlLanguage(), keymap.of([indentWithTab]), EditorView.lineWrapping];

type ClassifierBlockFormValues = Partial<ClassifierBlock> & {
  rulesYaml: string;
};

export function ListClassifierBlocks() {
  const toast = useRef(null);
  const deleteClassifierBlock = useDeleteClassifierBlock();
  const [listParams, setListParams] = useState<ListParams>({ sf: 'order' });
  const navigate = useNavigate();
  const { isPending, data, isFetching } = useClassifierBlocks(listParams);

  const nameTemplate = (block: ClassifierBlock) => (
    <Link className="title" to={`${block.id}/edit`}>{block.name}</Link>
  );

  const enabledTemplate = (block: ClassifierBlock) => (
    <span>{block.enabled ? 'Yes' : 'No'}</span>
  );

  const actionTemplate = (block: ClassifierBlock) => (
    <div className="flex flex-wrap gap-2">
      <Button type="button" icon="pi pi-pencil" severity="success" rounded text raised aria-description="Edit"
        onClick={() => navigate(`${block.id}/edit`)}
      ></Button>
      <Button type="button" icon="pi pi-trash" severity="danger" rounded text raised aria-description="Delete"
        onClick={() => confirmDeleteClassifierBlock(block)}
      ></Button>
    </div>
  );

  const doDeleteClassifierBlock = async (block: ClassifierBlock) => {
    await deleteClassifierBlock.mutateAsync(block.id, {
      onSuccess: () => {
        navigate('/classifier-blocks');
      },
    });
  };

  const confirmDeleteClassifierBlock = (block: ClassifierBlock) => {
    confirmDialog({
      message: 'Are you sure you want to delete this classifier block? Orders for later blocks will be compacted automatically.',
      header: `Delete: ${block.name}`,
      icon: 'pi pi-trash',
      defaultFocus: 'reject',
      acceptClassName: 'p-button-danger',
      accept: () => void doDeleteClassifierBlock(block),
    });
  };

  const onSort = (event: DataTableStateEvent) => {
    setListParams({ ...listParams, sf: event.sortField, sd: event.sortOrder === -1, page: 1 });
  };

  const onPage = (event: DataTableStateEvent) => {
    setListParams({ ...listParams, page: (event.page ?? 0) + 1, per_page: event.rows });
  };

  return (
    <>
      <Link to="new" style={{ float: 'right', padding: '1.5rem' }}>New Classifier &raquo;</Link>
      <Card title="Classifiers">
        <DataTable lazy value={data?.items}
          onPage={onPage}
          paginator={true}
          first={Math.max(((data?.page ?? listParams.page ?? 1) - 1) * (data?.per_page ?? listParams.per_page ?? 0), 0)}
          rows={data?.per_page ?? listParams.per_page}
          totalRecords={data?.total}
          loading={isPending || isFetching}
          onSort={onSort} sortField={listParams.sf} sortOrder={listParams.sd === true ? -1 : 1}
        >
          <Column field="order" header="Order" sortable></Column>
          <Column field="name" header="Name" body={nameTemplate} sortable></Column>
          <Column field="enabled" header="Enabled" body={enabledTemplate} sortable></Column>
          <Column field="description" header="Description" sortable></Column>
          <Column body={actionTemplate} headerClassName="w-9rem" />
        </DataTable>
      </Card>
      <Toast ref={toast} />
      <ConfirmDialog />
    </>
  );
}

export function EditClassifierBlock() {
  const id = useId('id');
  const { isLoading, isError, data, error } = useClassifierBlock(id);

  if (!id) {
    return <Message severity="error" text="Missing or invalid ID" />;
  }
  if (isError) {
    return <Message severity="error" text={error.message} />;
  }
  if (isLoading) {
    return <div>Loading</div>;
  }

  return (
    <Card title="Edit Classifier">
      {!isLoading && !isError && <ClassifierBlockForm data={data} />}
    </Card>
  );
}

export function NewClassifierBlock() {
  return (
    <Card title="New Classifier">
      <ClassifierBlockForm />
    </Card>
  );
}

function ClassifierBlockForm({ data }: Readonly<{ data?: Partial<ClassifierBlock> }>) {
  const saveClassifierBlock = useSaveClassifierBlock();
  const navigate = useNavigate();
  const formValues = useMemo(() => ({
    ...data,
    description: data?.description ?? '',
    enabled: data?.enabled ?? true,
    rulesYaml: rulesToYaml(data?.rules ?? defaultClassifierRules),
  }), [data]);
  const {
    control,
    handleSubmit,
    reset,
    setValue,
    formState: { errors, isSubmitting, isValid, isDirty },
  } = useForm<ClassifierBlockFormValues>({
    mode: 'onChange',
    defaultValues: {
      enabled: true,
      description: '',
      rulesYaml: rulesToYaml(defaultClassifierRules),
    },
  });

  const rulesYaml = useWatch({
    control,
    name: 'rulesYaml',
  }) ?? '';

  const parsedRules = useMemo(() => yamlToRules(rulesYaml), [rulesYaml]);
  const rulesError = parsedRules.error ?? null;

  useEffect(() => {
    if (isDirty) return;
    reset(formValues);
  }, [formValues, isDirty, reset]);

  const submitter = async (formData: ClassifierBlockFormValues) => {
    const parsed = yamlToRules(formData.rulesYaml);
    if (!parsed.value) {
      return;
    }

    await saveClassifierBlock.mutateAsync({
      id: formData.id,
      name: formData.name,
      description: formData.description || undefined,
      enabled: formData.enabled,
      rules: parsed.value,
    }, {
      onSuccess: () => {
        navigate('/classifier-blocks');
      },
    });
  };

  const errMsg = (name: keyof ClassifierBlockFormValues) =>
    errors[name]?.message ? String(errors[name]?.message) : null;

  const handleFormatYaml = () => {
    const parsed = yamlToRules(rulesYaml);
    if (!parsed.value) {
      return;
    }

    setValue('rulesYaml', rulesToYaml(parsed.value), {
      shouldDirty: true,
      shouldTouch: true,
      shouldValidate: true,
    });
  };

  const handleResetTemplate = () => {
    setValue('rulesYaml', rulesToYaml(defaultClassifierRules), {
      shouldDirty: true,
      shouldTouch: true,
      shouldValidate: true,
    });
  };

  return (
    <form onSubmit={handleSubmit(submitter)}>
      <div className="grid p-fluid">
        <div className="col-12 md:col-6 lg:col-4">
          <label htmlFor="name" className="font-medium mb-2 block">Name</label>
          <Controller name="name" control={control}
            rules={{
              required: 'Name is required',
              minLength: { value: 2, message: 'Name must be at least 2 characters' },
            }}
            render={({ field }) => (
              <InputText id="name" {...field}
                className={classNames({ 'p-invalid': !!errors.name })}
                placeholder="Classifier block name"
                autoComplete="name"
              />
            )}
          />
          {errMsg('name') && <small className="p-error">{errMsg('name')}</small>}
        </div>

        <div className="col-12 md:col-6 lg:col-4">
          <label htmlFor="order" className="font-medium mb-2 block">Order</label>
          <InputText id="order" value={data?.order?.toString() ?? 'Assigned automatically on create'} disabled />
          <small className="text-600">Order is managed by the API.</small>
        </div>

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
        </div>

        <div className="col-12 md:col-8">
          <label htmlFor="description" className="font-medium mb-2 block">Description</label>
          <Controller name="description" control={control}
            render={({ field }) => (
              <InputText id="description" {...field}
                className={classNames({ 'p-invalid': !!errors.description })}
                placeholder="Short description"
                autoComplete="description"
              />
            )}
          />
          {errMsg('description') && <small className="p-error">{errMsg('description')}</small>}
        </div>

        <div className="col-12">
          <div className="flex flex-wrap align-items-center justify-content-between gap-2 mb-2">
            <label htmlFor="rulesYaml" className="font-medium block mb-0">Rules</label>
            <div className="flex flex-wrap gap-2">
              <Button type="button" label="Format YAML" icon="pi pi-align-left" severity="secondary" text onClick={handleFormatYaml} disabled={!!rulesError} />
              {!data?.id && <Button type="button" label="Reset to template" icon="pi pi-refresh" severity="secondary" text onClick={handleResetTemplate} />}
            </div>
          </div>

          <Message severity="info" text="Edit classifier rules as YAML. They will be validated and saved as JSON." className="mb-3" />

          <Controller name="rulesYaml" control={control}
            rules={{
              required: 'Rules are required',
            }}
            render={({ field }) => (
              <CodeMirror
                id="rulesYaml"
                value={field.value ?? ''}
                height="24rem"
                theme={vscodeDark}
                extensions={yamlEditorExtensions}
                basicSetup={{
                  lineNumbers: true,
                  foldGutter: true,
                  highlightActiveLine: true,
                  highlightActiveLineGutter: true,
                }}
                onChange={(value) => field.onChange(value)}
                className={classNames('aut-yaml-editor', { 'is-invalid': !!errors.rulesYaml || !!rulesError })}
                placeholder="match_patterns: []\nmatch_actions: {}\nchild_rules: []"
              />
            )}
          />
          {errMsg('rulesYaml') && <small className="p-error block mt-2">{errMsg('rulesYaml')}</small>}
          {rulesError && <small className="p-error block mt-2">{rulesError}</small>}
        </div>
      </div>

      <div className="text-end">
        {saveClassifierBlock.isError && (
          <Message className="float-start" severity="error" text={saveClassifierBlock.error.message} />
        )}

        <Button label="Save" type="submit" icon="pi pi-check" raised disabled={!isDirty || !isValid || isSubmitting || !!rulesError} />
        <Button label="Cancel" type="button" severity="secondary" icon="pi pi-times" raised onClick={() => navigate('/classifier-blocks')} />
      </div>
    </form>
  );
}
