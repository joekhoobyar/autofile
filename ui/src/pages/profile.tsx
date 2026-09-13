import { useRef, type RefObject } from "react";
import { Controller, useForm } from "react-hook-form";
import { useNavigate } from "react-router-dom";

import { Button } from "primereact/button";
import { Card } from "primereact/card";
import { InputText } from "primereact/inputtext";
import { Password } from "primereact/password";
import { Message } from "primereact/message";
import { type Toast } from "primereact/toast";
import { classNames } from "primereact/utils";

import { AppToast } from "../components/AppToast";
import { PasswordStrengthMeter } from "../components/PasswordStrengthMeter";
import type { UserRole } from "../models/auth";
import type { PasswordChangeInput, ProfileUpdateInput } from "../models/profile";
import type { User } from "../models/user";
import { useChangePassword, useProfile, useSaveProfile } from "../queries/useProfile";

function formatRole(role: UserRole): string {
  return role === "admin" ? "Admin" : "User";
}

type ProfileFormValues = {
  username: string;
  role: UserRole;
  email: string;
  display_name: string;
};

type PasswordFormValues = {
  new_password: string;
  confirm_password: string;
};

export function Profile({ passwordOnly = false }: Readonly<{ passwordOnly?: boolean }>) {
  const navigate = useNavigate();
  const toast = useRef<Toast>(null);
  const { data, isLoading, isError, error } = useProfile({ enabled: !passwordOnly });

  if (isError) {
    return <Message severity="error" text={error.message} />;
  }

  if (!passwordOnly && (isLoading || !data)) {
    return <div>Loading</div>;
  }

  return (
    <>
      <div className="grid">
        {!passwordOnly && (
          <div className="col-12 xl:col-7">
            <ProfileDetailsForm data={data!} toast={toast} />
          </div>
        )}
        <div className={passwordOnly ? "col-12 xl:col-5" : "col-12 xl:col-5"}>
          {passwordOnly && (
            <Message
              severity="warn"
              text="You must change your password before continuing."
              className="mb-3 w-full"
            />
          )}
          <PasswordChangeForm toast={toast} onSuccess={() => passwordOnly && navigate("/documents", { replace: true })} />
        </div>
      </div>
      <AppToast ref={toast} />
    </>
  );
}

function ProfileDetailsForm({ data, toast }: Readonly<{ data: User; toast: RefObject<Toast | null> }>) {
  const saveProfile = useSaveProfile();
  const {
    control,
    handleSubmit,
    formState: { errors, isSubmitting, isValid, isDirty },
  } = useForm<ProfileFormValues>({
    mode: "onChange",
    defaultValues: {
      username: data.username,
      role: data.role,
      email: data.email,
      display_name: data.display_name,
    },
    values: {
      username: data.username,
      role: data.role,
      email: data.email,
      display_name: data.display_name,
    },
  });

  const submitter = async (values: ProfileFormValues) => {
    const input: ProfileUpdateInput = {
      email: values.email,
      display_name: values.display_name,
    };

    await saveProfile.mutateAsync(input, {
      onSuccess: () => {
        toast.current?.show({ severity: "success", summary: "Profile saved" });
      },
    });
  };

  const errMsg = (name: keyof ProfileFormValues) =>
    errors[name]?.message ? String(errors[name]?.message) : null;

  return (
    <Card title="Profile">
      <form onSubmit={handleSubmit(submitter)}>
        <div className="grid p-fluid">
          <div className="col-12 md:col-6">
            <label htmlFor="profile_username" className="font-medium mb-2 block">
              Username
            </label>
            <Controller
              name="username"
              control={control}
              render={({ field }) => <InputText id="profile_username" {...field} disabled />}
            />
          </div>

          <div className="col-12 md:col-6">
            <label htmlFor="profile_role" className="font-medium mb-2 block">
              Role
            </label>
            <Controller
              name="role"
              control={control}
              render={({ field }) => <InputText id="profile_role" value={formatRole(field.value)} disabled />}
            />
          </div>

          <div className="col-12 md:col-6">
            <label htmlFor="profile_display_name" className="font-medium mb-2 block">
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
                  id="profile_display_name"
                  {...field}
                  className={classNames({ "p-invalid": !!errors.display_name })}
                  placeholder="Display name"
                  autoComplete="name"
                />
              )}
            />
            {errMsg("display_name") && <small className="p-error">{errMsg("display_name")}</small>}
          </div>

          <div className="col-12 md:col-6">
            <label htmlFor="profile_email" className="font-medium mb-2 block">
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
                  id="profile_email"
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
          {saveProfile.isError && <Message className="float-start" severity="error" text={saveProfile.error.message} />}
          <Button
            label="Save Profile"
            type="submit"
            icon="pi pi-check"
            raised
            disabled={!isDirty || !isValid || isSubmitting}
          />
        </div>
      </form>
    </Card>
  );
}

function PasswordChangeForm({
  toast,
  onSuccess,
}: Readonly<{ toast: RefObject<Toast | null>; onSuccess?: () => void }>) {
  const changePassword = useChangePassword();
  const {
    control,
    handleSubmit,
    getValues,
    reset,
    formState: { errors, isSubmitting, isValid, isDirty },
  } = useForm<PasswordFormValues>({
    mode: "onChange",
    defaultValues: {
      new_password: "",
      confirm_password: "",
    },
  });

  const submitter = async (values: PasswordFormValues) => {
    const input: PasswordChangeInput = {
      new_password: values.new_password,
    };

    await changePassword.mutateAsync(input, {
      onSuccess: () => {
        reset();
        toast.current?.show({ severity: "success", summary: "Password changed" });
        onSuccess?.();
      },
    });
  };

  const errMsg = (name: keyof PasswordFormValues) =>
    errors[name]?.message ? String(errors[name]?.message) : null;

  return (
    <Card title="Change Password">
      <form onSubmit={handleSubmit(submitter)}>
        <div className="grid p-fluid">
          <div className="col-12">
            <label htmlFor="new_password" className="font-medium mb-2 block">
              New Password
            </label>
            <Controller
              name="new_password"
              control={control}
              rules={{
                required: "New password is required",
                minLength: { value: 12, message: "Password must be at least 12 characters" },
              }}
              render={({ field }) => (
                <>
                  <PasswordStrengthMeter value={field.value} />
                  <Password
                    inputId="new_password"
                    value={field.value ?? ""}
                    onChange={(e) => field.onChange(e.target.value)}
                    onBlur={field.onBlur}
                    inputRef={field.ref}
                    name={field.name}
                    className="aut-password"
                    inputClassName={classNames({ "p-invalid": !!errors.new_password })}
                    placeholder="New password"
                    autoComplete="new-password"
                    feedback={false}
                    toggleMask
                    invalid={!!errors.new_password}
                  />
                </>
              )}
            />
            {errMsg("new_password") && <small className="p-error">{errMsg("new_password")}</small>}
          </div>

          <div className="col-12">
            <label htmlFor="confirm_password" className="font-medium mb-2 block">
              Confirm New Password
            </label>
            <Controller
              name="confirm_password"
              control={control}
              rules={{
                required: "Confirm your new password",
                validate: (value) => value === getValues("new_password") || "Passwords do not match",
              }}
              render={({ field }) => (
                <InputText
                  id="confirm_password"
                  {...field}
                  type="password"
                  className={classNames({ "p-invalid": !!errors.confirm_password })}
                  placeholder="Confirm new password"
                  autoComplete="new-password"
                />
              )}
            />
            {errMsg("confirm_password") && <small className="p-error">{errMsg("confirm_password")}</small>}
          </div>
        </div>

        <div className="text-end">
          {changePassword.isError && <Message className="float-start" severity="error" text={changePassword.error.message} />}
          <Button
            label="Change Password"
            type="submit"
            icon="pi pi-key"
            raised
            disabled={!isDirty || !isValid || isSubmitting}
          />
        </div>
      </form>
    </Card>
  );
}
