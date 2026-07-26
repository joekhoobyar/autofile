import { useMemo, useRef, useState } from 'react';
import { useForm, Controller } from 'react-hook-form';
import { Link, useNavigate } from 'react-router-dom';

import { DataTable, type DataTableStateEvent } from 'primereact/datatable';
import { Column } from 'primereact/column';
import { Card } from 'primereact/card';
import { Button } from 'primereact/button';
import { InputText } from 'primereact/inputtext';
import { classNames } from 'primereact/utils';
import { Badge } from 'primereact/badge';

import type { ListParams } from '../api';
import { useTag, useTags, useSaveTag, useDeleteTag } from '../queries/useTags';
import { type Tag } from '../models/tag';
import { Message } from 'primereact/message';
import { useId } from '../util';
import { ConfirmDialog, confirmDialog } from 'primereact/confirmdialog';
import { ColorPicker } from 'primereact/colorpicker';
import { createSlugRules, normalizeSlug } from '../util/slugValidation';
import { useHashListParams } from '../util/listParamsHash';
import { AppToast } from '../components/AppToast';

const TAG_LIST_DEFAULT_PARAMS: ListParams = { sf: 'name' };

export function ListTags() {
  const toast = useRef(null);
  const deleteTag = useDeleteTag();
  const { listParams, updateListParams } = useHashListParams(TAG_LIST_DEFAULT_PARAMS);
  const appliedSearchText = listParams.q ?? '';
  const [searchDraft, setSearchDraft] = useState({ appliedSearchText, value: appliedSearchText });
  const searchText = searchDraft.appliedSearchText === appliedSearchText ? searchDraft.value : appliedSearchText;
  const setSearchText = (value: string) => setSearchDraft({ appliedSearchText, value });
  const navigate = useNavigate();
  const { isPending, data, isFetching } = useTags(listParams);

  const slugTemplate = (c: Tag) => {
    return (
      <Link className="title" to={`${c.id}/documents`}>{c.slug}</Link>
    );
  }

  const nameTemplate = (c: Tag) => {
    return (
      <Link className="title" to={`${c.id}/documents`}>{c.name}</Link>
    );
  }

  const previewTemplate = (c: Tag) => {
    return (
      <Link to={`${c.id}/documents`}>
        <Badge value={c.name} className="aut-document-tag" style={{ backgroundColor: `#${c.color}` }} />
      </Link>
    );
  };

  const actionTemplate = (c: Tag) => {
    return (
      <div className="flex flex-wrap gap-2">
        <Button type="button" icon="pi pi-pencil" severity="success" rounded text raised aria-description="Edit"
          onClick={() => navigate(`${c.id}/edit`)}
        ></Button>
        <Button type="button" icon="pi pi-trash" severity="danger" rounded text raised aria-description="Delete"
          onClick={() => confirmDeleteTag(c)}
        ></Button>
      </div>
    );
  };

  const doDeleteTag = async (c: Tag) => {
    await deleteTag.mutateAsync(c.id, {
      onSuccess: () => {
        navigate('/tags');
      }
    });
  }

  const confirmDeleteTag = (c: Tag) => {
    confirmDialog({
      message: 'Are you sure want to delete this tag?  All related documents will be untagged.',
      header: `Delete: ${c.name}`,
      icon: 'pi pi-trash',
      defaultFocus: 'reject',
      acceptClassName: 'p-button-danger',
      accept: () => void doDeleteTag(c),
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

  return (
    <>
    <Link to="new" style={{float: 'right', padding: '1.5rem'}}>New Tag &raquo;</Link>
    <Card title="Tags">
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
            placeholder="Search slug or name"
            aria-label="Search tags"
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
        <Column field="document_count" header="Documents"></Column>
        <Column header="Preview" body={previewTemplate}></Column>
        <Column body={actionTemplate} headerClassName="w-9rem" />
      </DataTable>
    </Card>
    <AppToast ref={toast} />
    <ConfirmDialog />
    </>
  );
}

export function EditTag() {
  const id = useId('id');
  const { isLoading, isError, data, error } = useTag(id);

  if (!id)
    return <Message severity="error" text="Missing or invalid ID" />;
  if (isError)
    return <Message severity="error" text={error.message} />
  if (isLoading)
    return <div>Loading</div>;

  return (
    <Card title="Edit Tag">
      { !isLoading && !isError && <TagForm data={data} /> }
    </Card>
  );
}

export function NewTag() {
  return (
    <Card title="New Tag">
      <TagForm />
    </Card>
  );
}

function TagForm({ data }: Readonly<{ data?: Partial<Tag> }>) {
  const saveTag = useSaveTag();
  const navigate = useNavigate();
  const {
    control,
    handleSubmit,
    formState: { errors, isSubmitting, isValid, isDirty },
  } = useForm<Partial<Tag>>({
    mode: 'onChange', // validate as user types
    defaultValues: {},
    values: data ?? {},
  });
  const slugRules = useMemo(
    () => createSlugRules('api/v1/tags/by-slug', !data?.id),
    [data?.id],
  );

  const submitter = async (data: Partial<Tag>) => {
    await saveTag.mutateAsync(data, {
      onSuccess: () => {
        navigate('/tags');
      }
    });
  };

  // PrimeReact-friendly error helper
  const errMsg = (name: keyof Partial<Tag>) =>
    errors[name]?.message ? String(errors[name]?.message) : null;

  return (
    <form onSubmit={handleSubmit(submitter)}>
      <div className="grid p-fluid">

        {/* Slug */}
        <div className="col-12 md:col-6 lg:col-4">
          <label htmlFor="slug" className="font-medium mb-2 block">Slug</label>
          <Controller name="slug" control={control}
            rules={slugRules}
            render={({ field }) => (
              <InputText id="slug" {...field}
                onChange={(event) => field.onChange(normalizeSlug(event.target.value))}
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

        {/* Color type */}
        <div className="col-12 md:col-6 lg:col-4">
          <label htmlFor="color" className="font-medium mb-2 block">Color</label>
          <Controller name="color" control={control}
            rules={{
              required: 'Color is required',
            }}
            render={({ field }) => (
              <ColorPicker id="data_type" {...field}
                className={classNames({ 'p-invalid': !!errors.color })}
              />
            )}
          />
          {errMsg('color') && <small className="p-error">{errMsg('color')}</small>}
        </div>
      </div>

      <div className="text-end">
        {saveTag.isError && (
          <Message className="float-start" severity="error" text={saveTag.error.message} />
        )}

        <Button label="Save" type="submit" icon="pi pi-check" raised disabled={!isDirty || !isValid || isSubmitting} />
        <Button label="Cancel" type="button" severity="secondary" icon="pi pi-times" raised onClick={() => navigate('/tags')} />
      </div>
    </form>
  );
}
