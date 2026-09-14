export interface AppSettings {
  id: number;
  allow_user_registration: boolean;
  date_format: string;
  datetime_format: string;
  virus_scanning_enabled: boolean;
  virus_scan_by_default: boolean;
  created_at: string;
  updated_at: string;
}

export interface AppSettingsUpdateInput {
  allow_user_registration: boolean;
  date_format: string;
  datetime_format: string;
  virus_scanning_enabled: boolean;
  virus_scan_by_default: boolean;
}

export interface PublicSettings {
  allow_user_registration: boolean;
  date_format: string;
  datetime_format: string;
  virus_scanning_enabled: boolean;
  virus_scan_by_default: boolean;
}
