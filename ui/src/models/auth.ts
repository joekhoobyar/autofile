export type UserRole = "admin" | "user";

export interface LoginRequest {
  username: string;
  password: string;
}

export interface LoginResult {
  role: UserRole;
}
