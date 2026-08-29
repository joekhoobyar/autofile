import { useEffect, useMemo, useRef, useState } from 'react';
import { Controller, useForm } from 'react-hook-form';
import { Link, useNavigate } from 'react-router-dom';

import { DataTable, type DataTableRowReorderEvent, type DataTableStateEvent } from 'primereact/datatable';
import { Column } from 'primereact/column';
import { Card } from 'primereact/card';
import { Button } from 'primereact/button';
import { InputText } from 'primereact/inputtext';
import { Checkbox } from 'primereact/checkbox';
import { Message } from 'primereact/message';
import { type Toast } from 'primereact/toast';
import { ConfirmDialog, confirmDialog } from 'primereact/confirmdialog';
import { classNames } from 'primereact/utils';
import type { ListParams } from '../api';
import { type ClassifierBlock, type ClassifierRules } from '../models/classifierBlock';
import { useClassifierBlock, useClassifierBlocks, useDeleteClassifierBlock, useReorderClassifierBlock, useSaveClassifierBlock, validateClassifierRules } from '../queries/useClassifierBlocks';
import { useId } from '../util';
import { defaultClassifierRules } from '../util/classifierRulesYaml';
import { useHashListParams } from '../util/listParamsHash';
import { AppToast } from '../components/AppToast';
import { ClassifierRulesEditor } from '../components/classifier/ClassifierRulesEditor';

const REORDER_SUCCESS_MS = 1200;
const CLASSIFIER_BLOCK_LIST_DEFAULT_PARAMS: ListParams = { sf: 'order' };

type ClassifierBlockFormValues = {
  id?: number;
  name: string;
  description: string;
  enabled: boolean;
  rules: ClassifierRules;
};

export function ListClassifierBlocks() {
  const toast = useRef<Toast>(null);
  const reorderSuccessTimeout = useRef<number | null>(null);
  const deleteClassifierBlock = useDeleteClassifierBlock();
  const reorderClassifierBlock = useReorderClassifierBlock();
  const { listParams, updateListParams } = useHashListParams(CLASSIFIER_BLOCK_LIST_DEFAULT_PARAMS);
  const appliedSearchText = listParams.q ?? '';
  const [searchDraft, setSearchDraft] = useState({ appliedSearchText, value: appliedSearchText });
  const searchText = searchDraft.appliedSearchText === appliedSearchText ? searchDraft.value : appliedSearchText;
  const setSearchText = (value: string) => setSearchDraft({ appliedSearchText, value });
  const [recentlyReorderedRowId, setRecentlyReorderedRowId] = useState<number | null>(null);
  const navigate = useNavigate();
  const { isPending, data, isFetching } = useClassifierBlocks(listParams);
  const canReorder = (listParams.sf ?? 'order') === 'order' && listParams.sd !== true;
  const pendingReorderRowId = reorderClassifierBlock.isPending ? (reorderClassifierBlock.variables?.id ?? null) : null;
  const firstRowIndex = Math.max(((data?.page ?? listParams.page ?? 1) - 1) * (data?.per_page ?? listParams.per_page ?? 0), 0);
  const tableItems = useMemo(
    () => data?.items?.map((block) => ({
      ...block,
      __pendingReorder: block.id === pendingReorderRowId,
      __recentlyReordered: block.id === recentlyReorderedRowId,
    })) ?? [],
    [data?.items, pendingReorderRowId, recentlyReorderedRowId],
  );

  useEffect(() => {
    return () => {
      if (reorderSuccessTimeout.current !== null) {
        globalThis.clearTimeout(reorderSuccessTimeout.current);
      }
    };
  }, []);

  const nameTemplate = (block: ClassifierBlock) => (
    <Link className="title" to={`${block.id}/edit`}>{block.name}</Link>
  );

  const enabledTemplate = (block: ClassifierBlock) => (
    <span>{block.enabled ? 'Yes' : 'No'}</span>
  );

  const orderTemplate = (block: ClassifierBlock & { __pendingReorder?: boolean; __recentlyReordered?: boolean }) => {
    const isPendingRow = block.__pendingReorder === true;
    const isSuccessRow = block.__recentlyReordered === true;

    return (
      <div className="aut-classifier-block-order-cell">
        <span>{block.order}</span>
        {isPendingRow && <span className="pi pi-spin pi-spinner aut-classifier-block-order-status" aria-hidden="true" />}
        {!isPendingRow && isSuccessRow && <span className="pi pi-check aut-classifier-block-order-status aut-classifier-block-order-status-success" aria-hidden="true" />}
      </div>
    );
  };

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
    try {
      await deleteClassifierBlock.mutateAsync(block.id);
      toast.current?.show({ severity: 'success', summary: 'Classifier deleted', detail: `Deleted ${block.name}.` });
    } catch (error) {
      const detail = error instanceof Error ? error.message : 'Something went wrong';
      toast.current?.show({ severity: 'error', summary: 'Delete failed', detail });
    }
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
    updateListParams({ ...listParams, sf: event.sortField, sd: event.sortOrder === -1, page: 1 });
  };

  const onPage = (event: DataTableStateEvent) => {
    updateListParams({ ...listParams, page: (event.page ?? 0) + 1, per_page: event.rows });
  };

  const applySearch = () => {
    updateListParams({
      ...listParams,
      q: searchText.trim() ? searchText.trim() : undefined,
      page: 1,
    });
  };

  const clearSearch = () => {
    setSearchText('');
    updateListParams({ ...listParams, q: undefined, page: 1 });
  };

  const onRowReorder = async (event: DataTableRowReorderEvent<ClassifierBlock[]>) => {
    if (!canReorder || pendingReorderRowId !== null) {
      return;
    }

    const movedBlock = data?.items?.[event.dragIndex];
    if (!movedBlock) {
      return;
    }

    const targetOrder = firstRowIndex + event.dropIndex + 1;
    if (targetOrder === movedBlock.order) {
      return;
    }

    try {
      setRecentlyReorderedRowId(null);
      await reorderClassifierBlock.mutateAsync({
        id: movedBlock.id,
        order: targetOrder,
      });

      if (reorderSuccessTimeout.current !== null) {
        globalThis.clearTimeout(reorderSuccessTimeout.current);
      }

      setRecentlyReorderedRowId(movedBlock.id);
      reorderSuccessTimeout.current = globalThis.setTimeout(() => {
        setRecentlyReorderedRowId(null);
        reorderSuccessTimeout.current = null;
      }, REORDER_SUCCESS_MS);
    } catch (error) {
      const detail = error instanceof Error ? error.message : 'Failed to reorder classifier block';
      toast.current?.show({ severity: 'error', summary: 'Reorder failed', detail });
    }
  };

  const rowClassName = (block: ClassifierBlock & { __pendingReorder?: boolean; __recentlyReordered?: boolean }) => classNames({
    'aut-classifier-block-row-pending': block.__pendingReorder === true,
    'aut-classifier-block-row-success': block.__recentlyReordered === true,
  });

  return (
    <>
      <Link to="new" style={{ float: 'right', padding: '1.5rem' }}>New Classifier &raquo;</Link>
      <Card title="Classifiers">
        {!canReorder && (
          <Message severity="info" text="Drag reordering is available only when sorted by order ascending." className="mb-3" />
        )}
        <div className="mb-3 w-full md:w-30rem">
          <div className="p-inputgroup w-full">
            <InputText
              value={searchText}
              onChange={(event) => setSearchText(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === 'Enter') {
                  event.preventDefault();
                  applySearch();
                }
              }}
              placeholder="Search name or description"
              aria-label="Search classifiers"
            />
            {searchText && (
              <span className="p-inputgroup-addon p-0">
                <Button
                  type="button"
                  icon="pi pi-times"
                  aria-label="Clear search"
                  onClick={clearSearch}
                  className="p-button-secondary h-full"
                  style={{ borderRadius: 0 }}
                />
              </span>
            )}
            <span className="p-inputgroup-addon p-0">
              <Button
                type="button"
                icon="pi pi-search"
                aria-label="Search"
                onClick={applySearch}
                className="p-button-info h-full"
                style={{ borderTopLeftRadius: 0, borderBottomLeftRadius: 0 }}
              />
            </span>
          </div>
        </div>

        <DataTable lazy value={tableItems}
          onPage={onPage}
          paginator={true}
          first={firstRowIndex}
          rows={data?.per_page ?? listParams.per_page}
          totalRecords={data?.total}
          loading={isPending || isFetching}
          onSort={onSort} sortField={listParams.sf} sortOrder={listParams.sd === true ? -1 : 1}
          reorderableRows={canReorder}
          onRowReorder={onRowReorder}
          rowClassName={rowClassName}
        >
          <Column rowReorder={canReorder} headerStyle={{ width: '3rem' }} bodyStyle={{ width: '3rem' }} />
          <Column field="order" header="Order" body={orderTemplate} sortable></Column>
          <Column field="name" header="Name" body={nameTemplate} sortable></Column>
          <Column field="enabled" header="Enabled" body={enabledTemplate} sortable></Column>
          <Column field="description" header="Description" sortable></Column>
          <Column body={actionTemplate} headerClassName="w-9rem" />
        </DataTable>
      </Card>
      <AppToast ref={toast} />
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
  const [rulesValid, setRulesValid] = useState(false);
  const [rulesSubmitError, setRulesSubmitError] = useState<string | null>(null);
  const formValues = useMemo(() => ({
    id: data?.id,
    name: data?.name ?? '',
    description: data?.description ?? '',
    enabled: data?.enabled ?? true,
    rules: data?.rules ?? structuredClone(defaultClassifierRules),
  }), [data]);
  const {
    control,
    handleSubmit,
    reset,
    formState: { errors, isSubmitting, isValid, isDirty },
  } = useForm<ClassifierBlockFormValues>({
    mode: 'onChange',
    defaultValues: {
      name: '',
      enabled: true,
      description: '',
      rules: structuredClone(defaultClassifierRules),
    },
  });

  useEffect(() => {
    if (isDirty) return;
    reset(formValues);
  }, [formValues, isDirty, reset]);

  const submitter = async (formData: ClassifierBlockFormValues) => {
    setRulesSubmitError(null);
    const validation = await validateClassifierRules(formData.rules);
    if (!validation.valid) {
      setRulesSubmitError(validation.issues[0]?.message ?? 'Invalid classifier rules');
      return;
    }

    await saveClassifierBlock.mutateAsync({
      id: formData.id,
      name: formData.name,
      description: formData.description || undefined,
      enabled: formData.enabled,
      rules: formData.rules,
    }, {
      onSuccess: () => {
        navigate('/classifier-blocks');
      },
    });
  };

  const errMsg = (name: keyof ClassifierBlockFormValues) =>
    errors[name]?.message ? String(errors[name]?.message) : null;

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

        <div className="col-12 mt-3">
          <Controller name="rules" control={control}
            render={({ field }) => (
              <ClassifierRulesEditor
                value={field.value}
                onChange={(rules) => field.onChange(rules)}
                onValidationChange={setRulesValid}
              />
            )}
          />
        </div>
      </div>

      <div className="text-end">
        {(saveClassifierBlock.isError || rulesSubmitError) && (
          <Message className="float-start" severity="error" text={rulesSubmitError ?? saveClassifierBlock.error?.message ?? 'Save failed'} />
        )}

        <Button label="Save" type="submit" icon="pi pi-check" raised disabled={!isDirty || !isValid || isSubmitting || !rulesValid} />
        <Button label="Cancel" type="button" severity="secondary" icon="pi pi-times" raised onClick={() => navigate('/classifier-blocks')} />
      </div>
    </form>
  );
}
