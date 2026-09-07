import type { UserRole } from "./auth";

export interface User {
  id: number;
  username: string;
  email: string;
  display_name: string;
  role: UserRole;
  force_password_change: boolean;
  created_at: string;
  updated_at: string;
  password_changed_at: string;
}

export interface UserUpdateInput {
  id: number;
  email?: string;
  display_name?: string;
  role?: UserRole;
  force_password_change?: boolean;
}
