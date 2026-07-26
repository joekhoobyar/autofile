import { useMemo, useRef, useState } from 'react';
import { useForm, Controller } from 'react-hook-form';
import { Link, useNavigate } from 'react-router-dom';

import { Column } from 'primereact/column';
import { Card } from 'primereact/card';
import { Button } from 'primereact/button';
import { InputText } from 'primereact/inputtext';
import { classNames } from 'primereact/utils';

import { useCabinet, useCabinetTree, useDeleteCabinet, useSaveCabinet } from '../queries/useCabinets';
import { type Cabinet } from '../models/cabinet';
import { Message } from 'primereact/message';
import { useId } from '../util';
import { TreeTable } from 'primereact/treetable';
import { TreeSelect } from 'primereact/treeselect';
import type { TreeNode } from 'primereact/treenode';
import { confirmDialog, ConfirmDialog } from 'primereact/confirmdialog';
import { type Toast } from 'primereact/toast';
import { createSlugRules, normalizeSlug } from '../util/slugValidation';
import { AppToast } from '../components/AppToast';

export function ListCabinets() {
  const toast = useRef<Toast>(null);
  const deleteCabinet = useDeleteCabinet();
  const [searchText, setSearchText] = useState('');
  const [activeSearch, setActiveSearch] = useState('');
  const navigate = useNavigate();
  const { isPending, data, isFetching } = useCabinetTree();

  const filteredData = useMemo(() => {
    const query = activeSearch.trim().toLowerCase();
    if (!query) {
      return data;
    }

    const filterNodes = (nodes: TreeNode[]): TreeNode[] => {
      const result: TreeNode[] = [];

      for (const node of nodes) {
        const children = node.children ? filterNodes(node.children) : [];
        const text = `${node.data.slug} ${node.data.name} ${node.data.description ?? ''}`.toLowerCase();
        const matchesSelf = text.includes(query);

        if (!matchesSelf && children.length === 0) {
          continue;
        }

        const filteredNode: TreeNode = {
          ...node,
          expanded: true,
          children,
          leaf: children.length === 0,
        };

        result.push(filteredNode);
      }

      return result;
    };

    return filterNodes(data ?? []);
  }, [activeSearch, data]);

  const slugTemplate = (c: TreeNode) => {
    return (
      <Link className="title" to={`${c.data.id}/documents`}>{c.data.slug}</Link>
    );
  }

  const nameTemplate = (c: TreeNode) => {
    return (
      <Link className="title" to={`${c.data.id}/documents`}>{c.data.name}</Link>
    );
  }

  const actionTemplate = (c: TreeNode) => {
    return (
      <div className="flex flex-wrap gap-2">
        <Button type="button" icon="pi pi-pencil" severity="success" rounded text raised aria-description="Edit"
          onClick={() => navigate(`${c.data.id}/edit`)}
        ></Button>
        <Button type="button" icon="pi pi-trash" severity="danger" rounded text raised aria-description="Delete"
          onClick={() => confirmDeleteCabinet(c.data)}
        ></Button>
      </div>
    );
  };

  const doDeleteCabinet = async (cabinet: Cabinet) => {
    try {
      await deleteCabinet.mutateAsync(cabinet.id);
      toast.current?.show({ severity: 'success', summary: 'Cabinet deleted', detail: `Deleted ${cabinet.name}.` });
    } catch (error) {
      const detail = error instanceof Error ? error.message : 'Something went wrong';
      toast.current?.show({ severity: 'error', summary: 'Delete failed', detail });
    }
  }

  const confirmDeleteCabinet = (cabinet: Cabinet) => {
    confirmDialog({
      message: 'Are you sure want to delete this cabinet?  No documents will be deleted.',
      header: `Delete: ${cabinet.name}`,
      icon: 'pi pi-trash',
      defaultFocus: 'reject',
      acceptClassName: 'p-button-danger',
      accept: () => void doDeleteCabinet(cabinet),
    });
  };

  const clearSearch = () => {
    setSearchText('');
    setActiveSearch('');
  };

  const applySearch = () => {
    setActiveSearch(searchText);
  };

  return (
    <>
    <Link to="new" style={{float: 'right', padding: '1.5rem'}}>New Cabinet &raquo;</Link>
    <Card title="Cabinets">
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
            placeholder="Search slug, name, or description"
            aria-label="Search cabinets"
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

      <TreeTable value={filteredData}
          loading={isPending || isFetching}
        >
        <Column field="slug" header="Slug" body={slugTemplate} sortable expander></Column>
        <Column field="name" header="Name" body={nameTemplate} sortable></Column>
        <Column field="document_count" header="Documents" sortable></Column>
        <Column field="description" header="Description" sortable></Column>
        <Column body={actionTemplate} headerClassName="w-9rem" />
      </TreeTable>
    </Card>
    <AppToast ref={toast} />
    <ConfirmDialog />
    </>
  );
}

export function EditCabinet() {
  const id = useId('id');
  const { isLoading, isError, data, error } = useCabinet(id);

  if (!id)
    return <Message severity="error" text="Missing or invalid ID" />;
  if (isError)
    return <Message severity="error" text={error.message} />
  if (isLoading)
    return <div>Loading</div>;

  return (
    <Card title="Edit Cabinet">
      { !isLoading && !isError && <CabinetForm data={data} /> }
    </Card>
  );
}

export function NewCabinet() {
  return (
    <Card title="New Cabinet">
      <CabinetForm />
    </Card>
  );
}

function CabinetForm({ data }: Readonly<{ data?: Partial<Cabinet> }>) {
  const saveCabinet = useSaveCabinet();
  const navigate = useNavigate();

  const {
    control,
    handleSubmit,
    formState: { errors, isSubmitting, isValid, isDirty },
  } = useForm<Partial<Cabinet>>({
    mode: 'onChange', // validate as user types
    defaultValues: {
    },
    values: data ?? {},
  });
  const slugRules = useMemo(
    () => createSlugRules('api/v1/cabinets/by-slug', !data?.id),
    [data?.id],
  );

  const submitter = async (data: Partial<Cabinet>) => {
    await saveCabinet.mutateAsync(data, {
      onSuccess: () => {
        navigate('/cabinets');
      }
    });
  };

  // PrimeReact-friendly error helper
  const errMsg = (name: keyof Partial<Cabinet>) =>
    errors[name]?.message ? String(errors[name]?.message) : null;

  const { data: parentOptions, isLoading: isParentsLoading } = useCabinetTree({ keyField: 'id' });
  const parentTreeOptions = useMemo(() => {
    const excludedId = data?.id;
    if (!excludedId) return parentOptions ?? [];

    const removeSubtree = (nodes: TreeNode[]): TreeNode[] => nodes
      .filter((node) => node.data.id !== excludedId)
      .map((node) => {
        const children = node.children ? removeSubtree(node.children) : [];
        return {
          ...node,
          children,
          leaf: children.length === 0,
        };
      });

    return removeSubtree(parentOptions ?? []);
  }, [data?.id, parentOptions]);

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

        {/* Parent */}
        <div className="col-12 md:col-6 lg:col-4">
          <label htmlFor="parent_id" className="font-medium mb-2 block">Parent Cabinet</label>
          <Controller name="parent_id" control={control}
            render={({ field }) => (
              <TreeSelect inputId="parent_id"
                value={field.value ? String(field.value) : null}
                onChange={(event) => field.onChange(event.value ? Number(event.value) : null)}
                className={classNames({ 'p-invalid': !!errors.parent_id })}
                placeholder="Parent cabinet"
                options={parentTreeOptions}
                disabled={isParentsLoading}
                filter
                showClear
              />
            )}
          />
          {errMsg('parent_id') && <small className="p-error">{errMsg('parent_id')}</small>}
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
        {saveCabinet.isError && (
          <Message className="float-start" severity="error" text={saveCabinet.error.message} />
        )}

        <Button label="Save" type="submit" icon="pi pi-check" raised disabled={!isDirty || !isValid || isSubmitting} />
        <Button label="Cancel" type="button" severity="secondary" icon="pi pi-times" raised onClick={() => navigate('/cabinets')} />
      </div>
    </form>
  );
}
