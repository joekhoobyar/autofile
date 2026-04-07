import { useCallback, useRef, useState } from 'react';
import { useForm, Controller } from 'react-hook-form';
import { Link, useNavigate } from 'react-router-dom';

import { DataTable, type DataTableStateEvent } from 'primereact/datatable';
import { Column } from 'primereact/column';
import { Card } from 'primereact/card';
import { Button } from 'primereact/button';
import { InputText } from 'primereact/inputtext';
import { Checkbox } from 'primereact/checkbox';
import { classNames } from 'primereact/utils';

import type { ListParams } from '../api';
import { useDeleteDocumentIndex, useDocumentIndex, useDocumentIndexes, useSaveDocumentIndex } from '../queries/useDocumentIndexes';
import { type DocumentIndex } from '../models/documentIndex';
import { Message } from 'primereact/message';
import { useId } from '../util';
import { Toast } from 'primereact/toast';
import { ConfirmDialog, confirmDialog } from 'primereact/confirmdialog';

export function ListDocumentIndexes() {
  const toast = useRef(null);
  const deleteDocumentIndex = useDeleteDocumentIndex();
  const [listParams, setListParams] = useState<ListParams>({sf: 'name'});
  const navigate = useNavigate();
  const { isPending, data, isFetching } = useDocumentIndexes(listParams);

  const slugTemplate = (c: DocumentIndex) => {
    return (
      <Link className="title" to={`${c.id}/values`}>{c.slug}</Link>
    );
  }

  const nameTemplate = (c: DocumentIndex) => {
    return (
      <Link className="title" to={`${c.id}/values`}>{c.name}</Link>
    );
  }

  const enabledTemplate = useCallback((rowData: {enabled: boolean}) => {
    return rowData.enabled ?
        <i className="pi pi-check" style={{color: 'var(--green-600)'}} /> :
        <i className="pi pi-times" style={{color: 'var(--red-600)'}} />;
  }, []);

  const actionTemplate = (c: DocumentIndex) => {
    return (
      <div className="flex flex-wrap gap-2">
        <Button type="button" icon="pi pi-folder-open" severity="info" rounded text raised aria-description="Edit"
          onClick={() => navigate(`${c.id}/templates`)}
        ></Button>
        <Button type="button" icon="pi pi-pencil" severity="success" rounded text raised aria-description="Edit"
          onClick={() => navigate(`${c.id}/edit`)}
        ></Button>
        <Button type="button" icon="pi pi-trash" severity="danger" rounded text raised aria-description="Delete"
          onClick={() => confirmDeleteDocumentIndex(c)}
        ></Button>
      </div>
    );
  };

  const doDeleteDocumentIndex = async (c: DocumentIndex) => {
    await deleteDocumentIndex.mutateAsync(c.id, {
      onSuccess: () => {
        navigate('/indexes');
      }
    });
  }

  const confirmDeleteDocumentIndex = (c: DocumentIndex) => {
    confirmDialog({
      message: 'Are you sure want to delete this document index?  All related index values will be deleted.',
      header: `Delete: ${c.name}`,
      icon: 'pi pi-trash',
      defaultFocus: 'reject',
      acceptClassName: 'p-button-danger',
      accept: () => void doDeleteDocumentIndex(c),
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
    <Link to="new" style={{float: 'right', padding: '1.5rem'}}>New Document Index &raquo;</Link>
    <Card title="Document Indexes">
      <DataTable lazy value={data?.items}
          onPage={onPage}
          paginator={true}
          first={Math.max(((data?.page ?? listParams.page ?? 1) - 1) * (data?.per_page ?? listParams.per_page ?? 0), 0)}
          rows={data?.per_page ?? listParams.per_page}
          totalRecords={data?.total}
          loading={isPending || isFetching}
          onSort={onSort} sortField={listParams.sf} sortOrder={listParams.sd===true ? -1 : 1}
        >
        <Column field="slug" header="Slug" body={slugTemplate} sortable></Column>
        <Column field="name" header="Name" body={nameTemplate} sortable></Column>
        <Column field="description" header="Description" sortable></Column>
        <Column field="enabled" header="Enabled" body={enabledTemplate} sortable></Column>
        <Column body={actionTemplate} headerClassName="w-13rem" />
      </DataTable>
    </Card>
    <Toast ref={toast} />
    <ConfirmDialog />
    </>
  );
}

export function EditDocumentIndex() {
  const id = useId('id');
  const { isLoading, isError, data, error } = useDocumentIndex(id);

  if (!id)
    return <Message severity="error" text="Missing or invalid ID" />;
  if (isError)
    return <Message severity="error" text={error.message} />
  if (isLoading)
    return <div>Loading</div>;

  return (
    <Card title="Edit Document Index">
      { !isLoading && !isError && <DocumentIndexForm data={data} /> }
    </Card>
  );
}

export function NewDocumentIndex() {
  return (
    <Card title="New Document Index">
      <DocumentIndexForm />
    </Card>
  );
}

function DocumentIndexForm({ data }: Readonly<{ data?: Partial<DocumentIndex> }>) {
  const saveDocumentIndex = useSaveDocumentIndex();
  const navigate = useNavigate();
  const {
    control,
    handleSubmit,
    formState: { errors, isSubmitting, isValid, isDirty },
  } = useForm<Partial<DocumentIndex>>({
    mode: 'onChange', // validate as user types
    defaultValues: {
      enabled: true,
    },
    values: data ?? {},
  });

  const submitter = async (data: Partial<DocumentIndex>) => {
    await saveDocumentIndex.mutateAsync(data, {
      onSuccess: () => {
        navigate('/indexes');
      }
    });
  };

  // PrimeReact-friendly error helper
  const errMsg = (name: keyof Partial<DocumentIndex>) =>
    errors[name]?.message ? String(errors[name]?.message) : null;

  return (
    <form onSubmit={handleSubmit(submitter)}>
      <div className="grid p-fluid">

        {/* Slug */}
        <div className="col-12 md:col-6 lg:col-4">
          <label htmlFor="slug" className="font-medium mb-2 block">Slug</label>
          <Controller name="slug" control={control}
            rules={{
              required: 'Slug is required',
              minLength: { value: 2, message: 'Slug must be at least 2 characters' },
            }}
            render={({ field }) => (
              <InputText id="slug" {...field}
                disabled={!!data?.id}
                className={classNames({ 'p-invalid': !!errors.slug })}
                placeholder="identifier" autoComplete="slug"
              />
            )}
          />
          {errMsg('slug') && <small className="p-error">{errMsg('slug')}</small>}
        </div>

        {/* Name */}
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
                placeholder="Short name" autoComplete="name"
              />
            )}
          />
          {errMsg('name') && <small className="p-error">{errMsg('name')}</small>}
        </div>

        {/* Description */}
        <div className="col-12 md:col-6">
          <label htmlFor="description" className="font-medium mb-2 block">Description</label>
          <Controller name="description" control={control}
            render={({ field }) => (
              <InputText id="description" {...field}
                className={classNames({ 'p-invalid': !!errors.description })}
                placeholder="Long name or description" autoComplete="description"
              />
            )}
          />
          {errMsg('description') && <small className="p-error">{errMsg('description')}</small>}
        </div>

        {/* Enabled */}
        <div className="col-12 md:col-6">
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
      </div>

      <div className="text-end">
        {saveDocumentIndex.isError && (
          <Message className="float-start" severity="error" text={saveDocumentIndex.error.message} />
        )}

        <Button label="Save" type="submit" icon="pi pi-check" raised disabled={!isDirty || !isValid || isSubmitting} />
        <Button label="Cancel" type="button" severity="secondary" icon="pi pi-times" raised onClick={() => navigate('/indexes')} />
      </div>
    </form>
  );
}
