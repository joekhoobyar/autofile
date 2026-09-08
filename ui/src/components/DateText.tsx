import type { ReactNode } from "react";
import { format } from "date-fns";

const DEFAULT_DATE_FORMAT = "MM/dd/yyyy HH:mm";

type DateTextProps = {
  value: Date | string | null | undefined;
  fallback?: ReactNode;
  formatString?: string;
};

export function DateText({ value, fallback = "", formatString = DEFAULT_DATE_FORMAT }: Readonly<DateTextProps>) {
  if (!value) {
    return <>{fallback}</>;
  }

  const date = value instanceof Date ? value : new Date(value);
  if (Number.isNaN(date.getTime())) {
    return <>{fallback}</>;
  }

  return <>{format(date, formatString)}</>;
}
