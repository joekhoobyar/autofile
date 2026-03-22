import { useRef, useState } from 'react';
import { useForm, Controller } from 'react-hook-form';
import { Link, useNavigate } from 'react-router-dom';

import { DataTable, type DataTableStateEvent } from 'primereact/datatable';
import { Column } from 'primereact/column';
import { Card } from 'primereact/card';
import { Button } from 'primereact/button';
import { InputText } from 'primereact/inputtext';
import { classNames } from 'primereact/utils';

import type { ListParams } from '../api';
import { useDeleteDocumentType, useDocumentType, useDocumentTypes, useSaveDocumentType } from '../queries/useDocumentTypes';
import { type DocumentType } from '../models/documentType';
import { Message } from 'primereact/message';
import { useId } from '../util';
import { Toast } from 'primereact/toast';
import { confirmDialog, ConfirmDialog } from 'primereact/confirmdialog';

export function ListDocumentTypes() {
  const toast = useRef(null);
  const deleteDocumentType = useDeleteDocumentType();
  const [listParams, setListParams] = useState<ListParams>({});
  const navigate = useNavigate();
  const { isPending, data, isFetching } = useDocumentTypes(listParams);

  const slugTemplate = (c: DocumentType) => {
    return (
      <Link className="title" to={`${c.id}/edit`}>{c.slug}</Link>
    );
  }

  const nameTemplate = (c: DocumentType) => {
    return (
      <Link className="title" to={`${c.id}/edit`}>{c.name}</Link>
    );
  }

  const actionTemplate = (c: DocumentType) => {
    return (
      <div className="flex flex-wrap gap-2">
        <Button size="small" type="button" icon="pi pi-pencil" severity="success" rounded aria-description="Edit"
          onClick={() => navigate(`${c.id}/edit`)}
        ></Button>
        <Button size="small" type="button" icon="pi pi-trash" severity="danger" rounded aria-description="Delete"
          onClick={() => confirmDeleteDocumentType(c)}
        ></Button>
      </div>
    );
  };

  const doDeleteDocumentType = async (c: DocumentType) => {
    await deleteDocumentType.mutateAsync(c.id, {
      onSuccess: () => {
        navigate('/document-types');
      }
    });
  }

  const confirmDeleteDocumentType = (c: DocumentType) => {
    confirmDialog({
      message: 'Are you sure want to delete this document type?  All related documents will be changed to the default type.',
      header: `Delete: ${c.name}`,
      icon: 'pi pi-trash',
      defaultFocus: 'reject',
      acceptClassName: 'p-button-danger',
      accept: () => void doDeleteDocumentType(c),
    });
  };

  const onSort = (event: DataTableStateEvent) => {
    setListParams({ ...listParams, sf: event.sortField, sd: event.sortOrder === -1 });
  };

  const onPage = (event: DataTableStateEvent) => {
    setListParams({ ...listParams, page: event.page, per_page: event.rows });
  };

  return (
    <>
    <Link to="new" style={{float: 'right', padding: '1.5rem'}}>New Document Type &raquo;</Link>
    <Card title="Document Types">
      <DataTable lazy value={data?.items}
          onPage={onPage} paginator={true} first={0} rows={data?.per_page} totalRecords={data?.total}
          loading={isPending || isFetching}
          onSort={onSort} sortField={listParams.sf} sortOrder={listParams.sd===true ? -1 : 1}
        >
        <Column field="slug" header="Slug" body={slugTemplate} sortable></Column>
        <Column field="name" header="Name" body={nameTemplate} sortable></Column>
        <Column field="description" header="Description" sortable></Column>
        <Column body={actionTemplate} headerClassName="w-9rem" />
      </DataTable>
    </Card>
    <Toast ref={toast} />
    <ConfirmDialog />
    </>
  );
}

export function EditDocumentType() {
  const id = useId('id');
  const { isLoading, isError, data, error } = useDocumentType(id);

  if (!id)
    return <Message severity="error" text="Missing or invalid ID" />;
  if (isError)
    return <Message severity="error" text={error.message} />
  if (isLoading)
    return <div>Loading</div>;

  return (
    <Card title="Edit Document Type">
      { !isLoading && !isError && <DocumentTypeForm data={data} /> }
    </Card>
  );
}

export function NewDocumentType() {
  return (
    <Card title="New Document Type">
      <DocumentTypeForm />
    </Card>
  );
}

function DocumentTypeForm({ data }: Readonly<{ data?: Partial<DocumentType> }>) {
  const saveDocumentType = useSaveDocumentType();
  const navigate = useNavigate();
  const {
    control,
    handleSubmit,
    formState: { errors, isSubmitting, isValid, isDirty },
  } = useForm<Partial<DocumentType>>({
    mode: 'onChange', // validate as user types
    defaultValues: {
    },
    values: data ?? {},
  });

  const submitter = async (data: Partial<DocumentType>) => {
    await saveDocumentType.mutateAsync(data, {
      onSuccess: () => {
        navigate('/document-types');
      }
    });
  };

  // PrimeReact-friendly error helper
  const errMsg = (name: keyof Partial<DocumentType>) =>
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
      </div>

      <div className="text-end">
        {saveDocumentType.isError && (
          <Message className="float-start" severity="error" text={saveDocumentType.error.message} />
        )}

        <Button label="Save" type="submit" icon="pi pi-check" disabled={!isDirty || !isValid || isSubmitting} />
        <Button label="Cancel" type="button" severity="secondary" icon="pi pi-times" style={{ marginLeft: '0.5em' }} onClick={() => navigate('/document-types')} />
      </div>
    </form>
  );
}