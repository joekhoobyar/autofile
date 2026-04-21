export interface User {
  id: number;
  username: string;
  email: string;
  display_name: string;
  created_at: string;
  updated_at: string;
  password_changed_at: string;
}

export interface UserUpdateInput {
  id: number;
  email?: string;
  display_name?: string;
}
