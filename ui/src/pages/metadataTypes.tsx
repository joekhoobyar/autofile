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
import { useMetadataType, useMetadataTypes, useSaveMetadataType, useDeleteMetadataType } from '../queries/useMetadataTypes';
import { type MetadataType } from '../models/metadataType';
import { Dropdown } from 'primereact/dropdown';
import { Message } from 'primereact/message';
import { useId } from '../util';
import { Toast } from 'primereact/toast';
import { ConfirmDialog, confirmDialog } from 'primereact/confirmdialog';

export function ListMetadataTypes() {
  const toast = useRef(null);
  const deleteMetadataType = useDeleteMetadataType();
  const [listParams, setListParams] = useState<ListParams>({});
  const navigate = useNavigate();
  const { isPending, data, isFetching } = useMetadataTypes(listParams);

  const slugTemplate = (c: MetadataType) => {
    return (
      <Link className="title" to={`${c.id}/edit`}>{c.slug}</Link>
    );
  }

  const nameTemplate = (c: MetadataType) => {
    return (
      <Link className="title" to={`${c.id}/edit`}>{c.name}</Link>
    );
  }

  const actionTemplate = (c: MetadataType) => {
    return (
      <div className="flex flex-wrap gap-2">
        <Button type="button" icon="pi pi-pencil" severity="success" rounded text raised aria-description="Edit"
          onClick={() => navigate(`${c.id}/edit`)}
        ></Button>
        <Button type="button" icon="pi pi-trash" severity="danger" rounded text raised aria-description="Delete"
          onClick={() => confirmDeleteMetadataType(c)}
        ></Button>
      </div>
    );
  };

  const doDeleteMetadataType = async (c: MetadataType) => {
    await deleteMetadataType.mutateAsync(c.id, {
      onSuccess: () => {
        navigate('/metadata-types');
      }
    });
  }

  const confirmDeleteMetadataType = (c: MetadataType) => {
    confirmDialog({
      message: 'Are you sure want to delete this metadata type?  All related document metadata will be deleted.',
      header: `Delete: ${c.name}`,
      icon: 'pi pi-trash',
      defaultFocus: 'reject',
      acceptClassName: 'p-button-danger',
      accept: () => void doDeleteMetadataType(c),
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
    <Link to="new" style={{float: 'right', padding: '1.5rem'}}>New Metadata Type &raquo;</Link>
    <Card title="Metadata Types">
      <DataTable lazy value={data?.items}
          onPage={onPage} paginator={true} first={0} rows={data?.per_page} totalRecords={data?.total}
          loading={isPending || isFetching}
          onSort={onSort} sortField={listParams.sf} sortOrder={listParams.sd===true ? -1 : 1}
        >
        <Column field="slug" header="Slug" body={slugTemplate} sortable></Column>
        <Column field="name" header="Name" body={nameTemplate} sortable></Column>
        <Column field="data_type" header="Data Type" sortable></Column>
        <Column field="description" header="Description" sortable></Column>
        <Column body={actionTemplate} headerClassName="w-9rem" />
      </DataTable>
    </Card>
    <Toast ref={toast} />
    <ConfirmDialog />
    </>
  );
}

export function EditMetadataType() {
  const id = useId('id');
  const { isLoading, isError, data, error } = useMetadataType(id);

  if (!id)
    return <Message severity="error" text="Missing or invalid ID" />;
  if (isError)
    return <Message severity="error" text={error.message} />
  if (isLoading)
    return <div>Loading</div>;

  return (
    <Card title="Edit Metadata Type">
      { !isLoading && !isError && <MetadataTypeForm data={data} /> }
    </Card>
  );
}

export function NewMetadataType() {
  return (
    <Card title="New Metadata Type">
      <MetadataTypeForm />
    </Card>
  );
}

function MetadataTypeForm({ data }: Readonly<{ data?: Partial<MetadataType> }>) {
  const saveMetadataType = useSaveMetadataType();
  const navigate = useNavigate();
  const {
    control,
    handleSubmit,
    formState: { errors, isSubmitting, isValid, isDirty },
  } = useForm<Partial<MetadataType>>({
    mode: 'onChange', // validate as user types
    defaultValues: {
      data_type: 'string',
    },
    values: data ?? {},
  });

  const submitter = async (data: Partial<MetadataType>) => {
    await saveMetadataType.mutateAsync(data, {
      onSuccess: () => {
        navigate('/metadata-types');
      }
    });
  };

  // PrimeReact-friendly error helper
  const errMsg = (name: keyof Partial<MetadataType>) =>
    errors[name]?.message ? String(errors[name]?.message) : null;

  const data_type_options = [
    { label: 'String', value: 'string' },
    { label: 'Integer', value: 'integer' },
    { label: 'Decimal', value: 'float' },
    { label: 'Boolean', value: 'boolean' },
    { label: 'Date', value: 'date' },
  ];

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

        {/* Data type */}
        <div className="col-12 md:col-6 lg:col-4">
          <label htmlFor="data_type" className="font-medium mb-2 block">Data Type</label>
          <Controller name="data_type" control={control}
            rules={{
              required: 'Data type is required',
            }}
            render={({ field }) => (
              <Dropdown id="data_type" {...field}
                className={classNames({ 'p-invalid': !!errors.data_type })}
                placeholder="Data type" options={data_type_options} autoComplete="data_type"
              />
            )}
          />
          {errMsg('data_type') && <small className="p-error">{errMsg('data_type')}</small>}
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
        {saveMetadataType.isError && (
          <Message className="float-start" severity="error" text={saveMetadataType.error.message} />
        )}

        <Button label="Save" type="submit" icon="pi pi-check" raised disabled={!isDirty || !isValid || isSubmitting} />
        <Button label="Cancel" type="button" severity="secondary" icon="pi pi-times" raised onClick={() => navigate('/metadata-types')} />
      </div>
    </form>
  );
}