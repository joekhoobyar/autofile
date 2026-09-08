import { useEffect, useRef, type RefObject } from "react";
import { Controller, useForm } from "react-hook-form";

import { Button } from "primereact/button";
import { Card } from "primereact/card";
import { Checkbox } from "primereact/checkbox";
import { Message } from "primereact/message";
import type { Toast } from "primereact/toast";

import { AppToast } from "../components/AppToast";
import type { AppSettingsUpdateInput } from "../models/appSettings";
import { useAppSettings, useSaveAppSettings } from "../queries/useAppSettings";

export function Settings() {
  const toast = useRef<Toast>(null);

  return (
    <>
      <SettingsForm toast={toast} />
      <AppToast ref={toast} />
    </>
  );
}

function SettingsForm({ toast }: Readonly<{ toast: RefObject<Toast | null> }>) {
  const { data, isPending, isError, error } = useAppSettings();
  const saveAppSettings = useSaveAppSettings();
  const {
    control,
    handleSubmit,
    reset,
    formState: { isDirty, isSubmitting },
  } = useForm<AppSettingsUpdateInput>({
    defaultValues: { allow_user_registration: true },
  });

  useEffect(() => {
    if (data) {
      reset({ allow_user_registration: data.allow_user_registration });
    }
  }, [data, reset]);

  const submitter = async (values: AppSettingsUpdateInput) => {
    try {
      const updated = await saveAppSettings.mutateAsync(values);
      reset({ allow_user_registration: updated.allow_user_registration });
      toast.current?.show({ severity: "success", summary: "Settings saved" });
    } catch (err) {
      const detail = err instanceof Error ? err.message : "Something went wrong";
      toast.current?.show({ severity: "error", summary: "Save failed", detail });
    }
  };

  return (
    <Card title="Settings">
      {isError && <Message severity="error" text={error.message} />}
      {isPending && <Message severity="info" text="Loading settings..." />}

      <form onSubmit={handleSubmit(submitter)}>
        <div className="p-fluid">
          <div className="col-12 md:col-8 lg:col-6">
            <Controller
              name="allow_user_registration"
              control={control}
              render={({ field }) => (
                <div className="flex align-items-center gap-2">
                  <Checkbox
                    inputId="allow_user_registration"
                    checked={field.value ?? false}
                    onChange={(event) => field.onChange(event.checked ?? false)}
                    disabled={isPending}
                  />
                  <label htmlFor="allow_user_registration">Allow user registration</label>
                </div>
              )}
            />
            <small className="block mt-2 text-color-secondary">
              When disabled, public requests to create an account are rejected.
            </small>
          </div>
        </div>

        <div className="text-end mt-3">
          {saveAppSettings.isError && (
            <Message className="float-start" severity="error" text={saveAppSettings.error.message} />
          )}
          <Button
            label="Save"
            type="submit"
            icon="pi pi-check"
            raised
            disabled={!isDirty || isSubmitting || isPending}
          />
        </div>
      </form>
    </Card>
  );
}
