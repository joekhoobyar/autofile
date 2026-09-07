export interface ProfileUpdateInput {
  email?: string;
  display_name?: string;
}

export interface PasswordChangeInput {
  new_password: string;
}
