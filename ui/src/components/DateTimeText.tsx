import type { ReactNode } from "react";

import { usePublicSettings } from "../queries/useAppSettings";
import { formatDateTime } from "../util/dateFormats";

type DateTimeTextProps = {
  value: Date | string | null | undefined;
  fallback?: ReactNode;
  formatString?: string;
};

export function DateTimeText({ value, fallback = "", formatString }: Readonly<DateTimeTextProps>) {
  const { data: settings } = usePublicSettings();
  const text = formatDateTime(value, formatString ?? settings?.datetime_format);

  if (!text) {
    return <>{fallback}</>;
  }

  return <>{text}</>;
}
