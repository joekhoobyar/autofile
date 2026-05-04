import { useState } from "react";
import { Controller, useForm } from "react-hook-form";
import { Link, useNavigate } from "react-router-dom";
import { format } from "date-fns";

import { DataTable, type DataTableStateEvent } from "primereact/datatable";
import { Column } from "primereact/column";
import { Card } from "primereact/card";
import { Button } from "primereact/button";
import { InputText } from "primereact/inputtext";
import { Message } from "primereact/message";
import { ConfirmDialog, confirmDialog } from "primereact/confirmdialog";
import { classNames } from "primereact/utils";

import type { ListParams } from "../api";
import { useId } from "../util";
import type { User, UserUpdateInput } from "../models/user";
import { useDeleteUser, useSaveUser, useUser, useUsers } from "../queries/useUsers";
import { canManageUsers, useAuth } from "../auth";
import { useHashListParams } from "../util/listParamsHash";

const SYSTEM_USER_ID = 1;
const USER_LIST_DEFAULT_PARAMS: ListParams = { sf: "username" };

function isSystemUser(id: number): boolean {
  return id === SYSTEM_USER_ID;
}

function formatDate(value: string): string {
  return format(new Date(value), "MM/dd/yyyy HH:mm");
}

export function ListUsers() {
  const auth = useAuth();
  const navigate = useNavigate();
  const deleteUser = useDeleteUser();
  const { listParams, updateListParams } = useHashListParams(USER_LIST_DEFAULT_PARAMS);
  const appliedSearchText = listParams.q ?? "";
  const [searchDraft, setSearchDraft] = useState({ appliedSearchText, value: appliedSearchText });
  const search = searchDraft.appliedSearchText === appliedSearchText ? searchDraft.value : appliedSearchText;
  const setSearch = (value: string) => setSearchDraft({ appliedSearchText, value });
  const { data, isPending, isFetching } = useUsers(listParams);

  if (!canManageUsers(auth)) {
    return <Message severity="warn" text="You do not have permission to manage users." />;
  }

  const onSort = (event: DataTableStateEvent) => {
    updateListParams({
      ...listParams,
      sf: event.sortField,
      sd: event.sortOrder === -1,
      page: 1,
    });
  };

  const onPage = (event: DataTableStateEvent) => {
    updateListParams({
      ...listParams,
      page: (event.page ?? 0) + 1,
      per_page: event.rows,
    });
  };

  const applySearch = () => {
    updateListParams({
      ...listParams,
      q: search.trim() ? search.trim() : undefined,
      page: 1,
    });
  };

  const clearSearch = () => {
    setSearch("");
    updateListParams({ ...listParams, q: undefined, page: 1 });
  };

  const confirmDeleteUser = (user: User) => {
    if (isSystemUser(user.id)) {
      return;
    }

    confirmDialog({
      message: "Are you sure you want to delete this user?",
      header: `Delete: ${user.username}`,
      icon: "pi pi-trash",
      defaultFocus: "reject",
      acceptClassName: "p-button-danger",
      accept: () => {
        void deleteUser.mutateAsync(user.id, {
          onSuccess: () => {
            navigate("/users");
          },
        });
      },
    });
  };

  const usernameTemplate = (user: User) => (
    <Link className="title" to={`${user.id}`}>
      {user.username}
    </Link>
  );

  const actionTemplate = (user: User) => (
    <div className="flex flex-wrap gap-2">
      <Button
        type="button"
        icon="pi pi-pencil"
        severity="success"
        rounded
        text
        raised
        aria-description="Edit"
        disabled={isSystemUser(user.id)}
        onClick={() => navigate(`${user.id}/edit`)}
      />
      <Button
        type="button"
        icon="pi pi-trash"
        severity="danger"
        rounded
        text
        raised
        aria-description="Delete"
        disabled={isSystemUser(user.id)}
        onClick={() => confirmDeleteUser(user)}
      />
    </div>
  );

  return (
    <>
      <Card title="Users">
        <div className="mb-3 w-full md:w-30rem">
          <div className="p-inputgroup w-full">
            <InputText
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              placeholder="Search username, display name, or email"
              aria-label="Search users"
              onKeyDown={(event) => {
                if (event.key === "Enter") {
                  event.preventDefault();
                  applySearch();
                }
              }}
            />
            {search && (
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

        <DataTable
          lazy
          value={data?.items}
          onPage={onPage}
          paginator
          first={Math.max(
            ((data?.page ?? listParams.page ?? 1) - 1) * (data?.per_page ?? listParams.per_page ?? 0),
            0,
          )}
          rows={data?.per_page ?? listParams.per_page}
          totalRecords={data?.total}
          loading={isPending || isFetching}
          onSort={onSort}
          sortField={listParams.sf}
          sortOrder={listParams.sd === true ? -1 : 1}
        >
          <Column field="username" header="Username" body={usernameTemplate} sortable />
          <Column field="display_name" header="Display Name" sortable />
          <Column field="email" header="Email" sortable />
          <Column field="password_changed_at" header="Password Changed" body={(u: User) => formatDate(u.password_changed_at)} sortable />
          <Column body={actionTemplate} headerClassName="w-9rem" />
        </DataTable>
      </Card>
      <ConfirmDialog />
    </>
  );
}

export function ViewUser() {
  const auth = useAuth();
  const id = useId("id");
  const navigate = useNavigate();
  const deleteUser = useDeleteUser();
  const { isLoading, isError, data, error } = useUser(id);

  if (!canManageUsers(auth)) {
    return <Message severity="warn" text="You do not have permission to manage users." />;
  }

  if (isError) {
    return <Message severity="error" text={error.message} />;
  }

  if (isLoading || !data) {
    return <div>Loading</div>;
  }

  const confirmDelete = () => {
    if (isSystemUser(data.id)) {
      return;
    }

    confirmDialog({
      message: "Are you sure you want to delete this user?",
      header: `Delete: ${data.username}`,
      icon: "pi pi-trash",
      defaultFocus: "reject",
      acceptClassName: "p-button-danger",
      accept: () => {
        void deleteUser.mutateAsync(data.id, {
          onSuccess: () => {
            navigate("/users");
          },
        });
      },
    });
  };

  return (
    <>
      <Card title={`User: ${data.username}`}>
        {isSystemUser(data.id) && (
          <Message severity="warn" text="System user cannot be edited or deleted." className="mb-3" />
        )}

        <ul className="aut-user-details">
          <li>
            <span>Username</span>: {data.username}
          </li>
          <li>
            <span>Display Name</span>: {data.display_name}
          </li>
          <li>
            <span>Email</span>: {data.email}
          </li>
          <li>
            <span>Created</span>: {formatDate(data.created_at)}
          </li>
          <li>
            <span>Updated</span>: {formatDate(data.updated_at)}
          </li>
          <li>
            <span>Password Changed</span>: {formatDate(data.password_changed_at)}
          </li>
        </ul>

        <div className="text-end">
          <Button
            label="Edit"
            type="button"
            icon="pi pi-pencil"
            raised
            disabled={isSystemUser(data.id)}
            onClick={() => navigate(`/users/${data.id}/edit`)}
          />
          <Button
            label="Delete"
            type="button"
            icon="pi pi-trash"
            severity="danger"
            raised
            disabled={isSystemUser(data.id)}
            onClick={confirmDelete}
          />
          <Button
            label="Back"
            type="button"
            severity="secondary"
            icon="pi pi-arrow-left"
            raised
            onClick={() => navigate("/users")}
          />
        </div>
      </Card>
      <ConfirmDialog />
    </>
  );
}

export function EditUser() {
  const auth = useAuth();
  const id = useId("id");
  const navigate = useNavigate();
  const { isLoading, isError, data, error } = useUser(id);

  if (!canManageUsers(auth)) {
    return <Message severity="warn" text="You do not have permission to manage users." />;
  }

  if (isError) {
    return <Message severity="error" text={error.message} />;
  }

  if (isLoading || !data) {
    return <div>Loading</div>;
  }

  if (isSystemUser(data.id)) {
    return (
      <Card title="Edit User">
        <Message severity="warn" text="System user cannot be edited." className="mb-3" />
        <div className="text-end">
          <Button
            label="Back"
            type="button"
            severity="secondary"
            icon="pi pi-arrow-left"
            raised
            onClick={() => navigate(`/users/${data.id}`)}
          />
        </div>
      </Card>
    );
  }

  return (
    <Card title="Edit User">
      <UserForm data={data} />
    </Card>
  );
}

type UserFormValues = {
  id: number;
  username: string;
  email: string;
  display_name: string;
};

function UserForm({ data }: Readonly<{ data: User }>) {
  const navigate = useNavigate();
  const saveUser = useSaveUser();
  const {
    control,
    handleSubmit,
    formState: { errors, isSubmitting, isValid, isDirty },
  } = useForm<UserFormValues>({
    mode: "onChange",
    defaultValues: {
      id: data.id,
      username: data.username,
      email: data.email,
      display_name: data.display_name,
    },
    values: {
      id: data.id,
      username: data.username,
      email: data.email,
      display_name: data.display_name,
    },
  });

  const submitter = async (values: UserFormValues) => {
    const input: UserUpdateInput = {
      id: values.id,
      email: values.email,
      display_name: values.display_name,
    };

    await saveUser.mutateAsync(input, {
      onSuccess: () => {
        navigate(`/users/${values.id}`);
      },
    });
  };

  const errMsg = (name: keyof UserFormValues) =>
    errors[name]?.message ? String(errors[name]?.message) : null;

  return (
    <form onSubmit={handleSubmit(submitter)}>
      <div className="grid p-fluid">
        <div className="col-12 md:col-6 lg:col-4">
          <label htmlFor="username" className="font-medium mb-2 block">
            Username
          </label>
          <Controller
            name="username"
            control={control}
            render={({ field }) => <InputText id="username" {...field} disabled />}
          />
        </div>

        <div className="col-12 md:col-6 lg:col-4">
          <label htmlFor="display_name" className="font-medium mb-2 block">
            Display Name
          </label>
          <Controller
            name="display_name"
            control={control}
            rules={{
              required: "Display name is required",
              minLength: { value: 2, message: "Display name must be at least 2 characters" },
            }}
            render={({ field }) => (
              <InputText
                id="display_name"
                {...field}
                className={classNames({ "p-invalid": !!errors.display_name })}
                placeholder="Display name"
                autoComplete="name"
              />
            )}
          />
          {errMsg("display_name") && <small className="p-error">{errMsg("display_name")}</small>}
        </div>

        <div className="col-12 md:col-6 lg:col-4">
          <label htmlFor="email" className="font-medium mb-2 block">
            Email
          </label>
          <Controller
            name="email"
            control={control}
            rules={{
              required: "Email is required",
              pattern: {
                value: /^[^\s@]+@[^\s@]+\.[^\s@]+$/,
                message: "Enter a valid email address",
              },
            }}
            render={({ field }) => (
              <InputText
                id="email"
                {...field}
                className={classNames({ "p-invalid": !!errors.email })}
                placeholder="Email"
                autoComplete="email"
              />
            )}
          />
          {errMsg("email") && <small className="p-error">{errMsg("email")}</small>}
        </div>
      </div>

      <div className="text-end">
        {saveUser.isError && <Message className="float-start" severity="error" text={saveUser.error.message} />}

        <Button
          label="Save"
          type="submit"
          icon="pi pi-check"
          raised
          disabled={!isDirty || !isValid || isSubmitting}
        />
        <Button
          label="Cancel"
          type="button"
          severity="secondary"
          icon="pi pi-times"
          raised
          onClick={() => navigate(`/users/${data.id}`)}
        />
      </div>
    </form>
  );
}
