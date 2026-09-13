import { format } from "date-fns";

export const DEFAULT_DATE_FORMAT = "yyyy-MM-dd";
export const DEFAULT_DATETIME_FORMAT = "MM/dd/yyyy HH:mm";

export type DateFormatOption = {
  value: string;
  example: string;
  calendarFormat: string;
};

export type DateTimeFormatOption = {
  value: string;
  example: string;
};

export const DATE_FORMAT_OPTIONS: DateFormatOption[] = [
  { value: "yyyy-MM-dd", example: "2026-08-25", calendarFormat: "yy-mm-dd" },
  { value: "MM/dd/yyyy", example: "08/25/2026", calendarFormat: "mm/dd/yy" },
  { value: "dd/MM/yyyy", example: "25/08/2026", calendarFormat: "dd/mm/yy" },
  { value: "dd.MM.yyyy", example: "25.08.2026", calendarFormat: "dd.mm.yy" },
  { value: "MMM d, yyyy", example: "Aug 25, 2026", calendarFormat: "M d, yy" },
];

export const DATETIME_FORMAT_OPTIONS: DateTimeFormatOption[] = [
  { value: "MM/dd/yyyy HH:mm", example: "08/25/2026 14:30" },
  { value: "MM/dd/yyyy h:mm a", example: "08/25/2026 2:30 PM" },
  { value: "yyyy-MM-dd HH:mm", example: "2026-08-25 14:30" },
  { value: "dd/MM/yyyy HH:mm", example: "25/08/2026 14:30" },
  { value: "dd.MM.yyyy HH:mm", example: "25.08.2026 14:30" },
  { value: "MMM d, yyyy h:mm a", example: "Aug 25, 2026 2:30 PM" },
];

export function getDateFormatOption(formatString: string | null | undefined) {
  return DATE_FORMAT_OPTIONS.find((option) => option.value === formatString) ?? DATE_FORMAT_OPTIONS[0];
}

export function getDateTimeFormatOption(formatString: string | null | undefined) {
  return DATETIME_FORMAT_OPTIONS.find((option) => option.value === formatString) ?? DATETIME_FORMAT_OPTIONS[0];
}

export function parseBackendDate(value: string) {
  const [year, month, day] = value.split('-').map(Number);
  if (!year || !month || !day) return null;

  const date = new Date(year, month - 1, day);
  if (date.getFullYear() !== year || date.getMonth() !== month - 1 || date.getDate() !== day) return null;
  return date;
}

export function serializeBackendDate(value: Date) {
  return `${value.getFullYear()}-${String(value.getMonth() + 1).padStart(2, '0')}-${String(value.getDate()).padStart(2, '0')}`;
}

export function formatDateOnly(value: Date | string | null | undefined, formatString = DEFAULT_DATE_FORMAT) {
  if (!value) return null;

  const date = value instanceof Date ? value : parseBackendDate(value);
  if (!date || Number.isNaN(date.getTime())) return null;

  return format(date, formatString);
}

export function formatDateTime(value: Date | string | null | undefined, formatString = DEFAULT_DATETIME_FORMAT) {
  if (!value) return null;

  const date = value instanceof Date ? value : new Date(value);
  if (Number.isNaN(date.getTime())) return null;

  return format(date, formatString);
}
