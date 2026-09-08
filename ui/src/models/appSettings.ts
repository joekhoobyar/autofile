export interface AppSettings {
  id: number;
  allow_user_registration: boolean;
  created_at: string;
  updated_at: string;
}

export interface AppSettingsUpdateInput {
  allow_user_registration: boolean;
}
