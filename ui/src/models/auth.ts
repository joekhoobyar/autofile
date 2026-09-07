export type UserRole = "admin" | "user";

export interface AuthSession {
  userId: number;
  role: UserRole;
}

export interface LoginRequest {
  username: string;
  password: string;
}

export interface LoginResult {
  userId: number;
  role: UserRole;
}
