export type UserRole = "admin" | "user";

export interface AuthSession {
  userId: number;
  role: UserRole;
  forcePasswordChange: boolean;
}

export interface AccessTokenResponse {
  access_token: string;
  token_type: "Bearer";
  expires_in: number;
}

export interface LoginRequest {
  username: string;
  password: string;
}

export interface RegisterRequest {
  username: string;
  email: string;
  display_name: string;
  password: string;
}

export interface LoginResult {
  userId: number;
  role: UserRole;
  forcePasswordChange: boolean;
}
