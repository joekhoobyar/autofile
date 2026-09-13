import { useEffect, useRef, type RefObject } from "react";
import { Controller, useForm } from "react-hook-form";

import { Button } from "primereact/button";
import { Card } from "primereact/card";
import { Checkbox } from "primereact/checkbox";
import { Dropdown } from "primereact/dropdown";
import { Message } from "primereact/message";
import type { Toast } from "primereact/toast";

import { AppToast } from "../components/AppToast";
import type { AppSettingsUpdateInput } from "../models/appSettings";
import { useAppSettings, useSaveAppSettings } from "../queries/useAppSettings";
import { DATE_FORMAT_OPTIONS, DATETIME_FORMAT_OPTIONS } from "../util/dateFormats";

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
    defaultValues: {
      allow_user_registration: true,
      date_format: "yyyy-MM-dd",
      datetime_format: "MM/dd/yyyy HH:mm",
      virus_scanning_enabled: false,
      virus_scan_by_default: true,
    },
  });

  useEffect(() => {
    if (data) {
      reset({
        allow_user_registration: data.allow_user_registration,
        date_format: data.date_format,
        datetime_format: data.datetime_format,
        virus_scanning_enabled: data.virus_scanning_enabled,
        virus_scan_by_default: data.virus_scan_by_default,
      });
    }
  }, [data, reset]);

  const submitter = async (values: AppSettingsUpdateInput) => {
    try {
      const updated = await saveAppSettings.mutateAsync(values);
      reset({
        allow_user_registration: updated.allow_user_registration,
        date_format: updated.date_format,
        datetime_format: updated.datetime_format,
        virus_scanning_enabled: updated.virus_scanning_enabled,
        virus_scan_by_default: updated.virus_scan_by_default,
      });
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

          <div className="col-12 md:col-8 lg:col-6 mt-4">
            <Controller
              name="date_format"
              control={control}
              render={({ field }) => (
                <div className="field">
                  <label htmlFor="date_format" className="font-medium mb-2 block">Date format</label>
                  <Dropdown
                    inputId="date_format"
                    value={field.value}
                    onChange={(event) => field.onChange(event.value)}
                    options={DATE_FORMAT_OPTIONS}
                    optionLabel="example"
                    optionValue="value"
                    disabled={isPending}
                    className="w-full"
                  />
                  <small className="block mt-2 text-color-secondary">
                    Used for date metadata display and date picker input.
                  </small>
                </div>
              )}
            />
          </div>

          <div className="col-12 md:col-8 lg:col-6 mt-4">
            <Controller
              name="datetime_format"
              control={control}
              render={({ field }) => (
                <div className="field">
                  <label htmlFor="datetime_format" className="font-medium mb-2 block">Date/time format</label>
                  <Dropdown
                    inputId="datetime_format"
                    value={field.value}
                    onChange={(event) => field.onChange(event.value)}
                    options={DATETIME_FORMAT_OPTIONS}
                    optionLabel="example"
                    optionValue="value"
                    disabled={isPending}
                    className="w-full"
                  />
                  <small className="block mt-2 text-color-secondary">
                    Used for timestamps such as created and updated times.
                  </small>
                </div>
              )}
            />
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
