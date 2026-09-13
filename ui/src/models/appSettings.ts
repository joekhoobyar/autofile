export interface AppSettings {
  id: number;
  allow_user_registration: boolean;
  date_format: string;
  datetime_format: string;
  created_at: string;
  updated_at: string;
}

export interface AppSettingsUpdateInput {
  allow_user_registration: boolean;
  date_format: string;
  datetime_format: string;
}

export interface PublicSettings {
  allow_user_registration: boolean;
  date_format: string;
  datetime_format: string;
}
