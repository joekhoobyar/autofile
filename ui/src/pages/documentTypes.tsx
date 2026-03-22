import { useEffect, useMemo, useRef, useState } from 'react';
import { useForm, Controller, useWatch } from 'react-hook-form';
import { Link, useNavigate } from 'react-router-dom';

import { DataTable, type DataTableStateEvent } from 'primereact/datatable';
import { Column } from 'primereact/column';
import { Card } from 'primereact/card';
import { Button } from 'primereact/button';
import { InputText } from 'primereact/inputtext';
import { MultiSelect } from 'primereact/multiselect';
import { Checkbox } from 'primereact/checkbox';
import { classNames } from 'primereact/utils';

import type { ListParams } from '../api';
import { useDeleteDocumentType, useDocumentType, useDocumentTypes, useSaveDocumentType } from '../queries/useDocumentTypes';
import { type DocumentType } from '../models/documentType';
import { Message } from 'primereact/message';
import { useId } from '../util';
import { Toast } from 'primereact/toast';
import { confirmDialog, ConfirmDialog } from 'primereact/confirmdialog';
import { useDocumentTypeMetadataTypes, useDocumentTypeSaveMetadataTypes, useMetadataTypesMap } from '../queries/useMetadataTypes';
import { type DocumentTypeNewMetadataType } from '../models/documentTypeMetadataType';

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
        <Button type="button" icon="pi pi-pencil" severity="success" rounded text raised aria-description="Edit"
          onClick={() => navigate(`${c.id}/edit`)}
        ></Button>
        <Button type="button" icon="pi pi-trash" severity="danger" rounded text raised aria-description="Delete"
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

type DocumentTypeFormValues = Partial<DocumentType> & {
  metadata_type_ids: number[];
  metadata_type_required: Record<number, boolean>;
};

function DocumentTypeForm({ data }: Readonly<{ data?: Partial<DocumentType> }>) {
  const saveDocumentType = useSaveDocumentType();
  const { data: documentTypeMetadataTypes } = useDocumentTypeMetadataTypes(data?.id);
  const { data: metadataTypesMap, isLoading: isLoadingMetadataTypes } = useMetadataTypesMap('id');
  const [metadataSaveRequest, setMetadataSaveRequest] = useState<{
    documentTypeId: number;
    payload: DocumentTypeNewMetadataType[];
  } | null>(null);
  const saveMetadataTypes = useDocumentTypeSaveMetadataTypes(data?.id ?? metadataSaveRequest?.documentTypeId ?? 0);
  const navigate = useNavigate();
  const selectedMetadataTypeIds = useMemo(
    () => documentTypeMetadataTypes?.map((item) => item.metadata_type_id) ?? [],
    [documentTypeMetadataTypes]
  );
  const metadataTypeRequiredMap = useMemo(() => (
    documentTypeMetadataTypes?.reduce((acc, item) => {
      acc[item.metadata_type_id] = item.required;
      return acc;
    }, {} as Record<number, boolean>) ?? {}
  ), [documentTypeMetadataTypes]);
  const metadataTypeOptions = useMemo(() => {
    if (!metadataTypesMap) return [];
    return Object.values(metadataTypesMap)
      .sort((a, b) => (a.name ?? a.slug).localeCompare(b.name ?? b.slug))
      .map((item) => ({
        label: item.name ?? item.slug,
        value: item.id,
      }));
  }, [metadataTypesMap]);
  const formValues = useMemo(() => ({
    ...data,
    metadata_type_ids: selectedMetadataTypeIds,
    metadata_type_required: metadataTypeRequiredMap,
  }), [data, selectedMetadataTypeIds, metadataTypeRequiredMap]);
  const {
    control,
    handleSubmit,
    reset,
    formState: { errors, isSubmitting, isValid, isDirty },
  } = useForm<DocumentTypeFormValues>({
    mode: 'onChange', // validate as user types
    defaultValues: {
      metadata_type_ids: [],
      metadata_type_required: {},
    },
  });

  const watchedMetadataTypeIds = useWatch({
    control,
    name: 'metadata_type_ids',
  });

  useEffect(() => {
    if (isDirty) return;
    reset(formValues);
  }, [formValues, isDirty, reset]);

  useEffect(() => {
    if (!metadataSaveRequest) return;
    let cancelled = false;

    const run = async () => {
      try {
        await saveMetadataTypes.mutateAsync(metadataSaveRequest.payload);
        if (!cancelled) {
          setMetadataSaveRequest(null);
          navigate('/document-types');
        }
      } catch (err) {
        console.error(err);
        if (!cancelled) {
          setMetadataSaveRequest(null);
        }
      }
    };

    void run();
    return () => {
      cancelled = true;
    };
  }, [metadataSaveRequest, navigate, saveMetadataTypes]);

  const submitter = async (data: DocumentTypeFormValues) => {
    const { metadata_type_ids, metadata_type_required, ...documentTypeData } = data;
    const savedDocumentType = await saveDocumentType.mutateAsync(documentTypeData);
    const documentTypeId = savedDocumentType.id ?? documentTypeData.id;
    if (!documentTypeId) return;

    const payload = (metadata_type_ids ?? []).map((metadata_type_id) => ({
      metadata_type_id,
      required: metadata_type_required?.[metadata_type_id] ?? false,
    }));

    if (documentTypeData.id) {
      await saveMetadataTypes.mutateAsync(payload);
      navigate('/document-types');
      return;
    }

    setMetadataSaveRequest({ documentTypeId, payload });
  };

  // PrimeReact-friendly error helper
  const errMsg = (name: keyof DocumentTypeFormValues) =>
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

        {/* Metadata Types */}
        <div className="col-12">
          <label htmlFor="metadata_type_ids" className="font-medium mb-2 block">Metadata Types</label>
          <Controller name="metadata_type_ids" control={control}
            render={({ field }) => (
              <MultiSelect id="metadata_type_ids" value={field.value ?? []}
                options={metadataTypeOptions}
                onChange={(event) => field.onChange(event.value ?? [])}
                filter
                display="chip"
                optionLabel="label"
                optionValue="value"
                placeholder={metadataTypeOptions.length ? 'Select metadata types' : 'No metadata types available'}
                disabled={isLoadingMetadataTypes || !metadataTypeOptions.length}
                className={classNames({ 'p-invalid': !!errors.metadata_type_ids })}
              />
            )}
          />
          {errMsg('metadata_type_ids') && <small className="p-error">{errMsg('metadata_type_ids')}</small>}
        </div>

        {/* Required Metadata Types */}
        <div className="col-12">
          <label className="font-medium mb-2 block">Required Fields</label>
          <Controller name="metadata_type_required" control={control}
            render={({ field }) => {
              const selectedIds = new Set(watchedMetadataTypeIds ?? []);
              const selectedOptions = metadataTypeOptions.filter((option) => selectedIds.has(option.value));

              if (!selectedOptions.length) {
                return <small className="text-600">Select metadata types to set required fields.</small>;
              }

              return (
                <div className="flex flex-wrap gap-3">
                  {selectedOptions.map((option) => (
                    <div key={option.value} className="flex align-items-center gap-2">
                      <Checkbox
                        inputId={`metadata-required-${option.value}`}
                        checked={field.value?.[option.value] ?? false}
                        onChange={(event) => {
                          field.onChange({
                            ...field.value,
                            [option.value]: event.checked ?? false,
                          });
                        }}
                      />
                      <label htmlFor={`metadata-required-${option.value}`}>{option.label}</label>
                    </div>
                  ))}
                </div>
              );
            }}
          />
        </div>
      </div>

      <div className="text-end">
        {saveDocumentType.isError && (
          <Message className="float-start" severity="error" text={saveDocumentType.error.message} />
        )}
        {saveMetadataTypes.isError && (
          <Message className="float-start" severity="error" text={saveMetadataTypes.error.message} />
        )}

        <Button label="Save" type="submit" icon="pi pi-check" raised disabled={!isDirty || !isValid || isSubmitting || saveMetadataTypes.isPending} />
        <Button label="Cancel" type="button" severity="secondary" icon="pi pi-times" raised onClick={() => navigate('/document-types')} />
      </div>
    </form>
  );
}
