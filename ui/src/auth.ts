import React from "react";
import { apiMutate, setAccessToken } from "./api";
import type { LoginRequest } from "./models/auth";

export async function login(user: LoginRequest) {
  const data = await apiMutate<{ access_token: string }>("api/v1/auth/login", {
    method: 'POST',
    body: user,
    retryOn401: false,
  });
  setAccessToken(data.access_token);
}

export async function logout() {
  await apiMutate<void>("api/v1/auth/logout", {
    method: 'POST',
    retryOn401: false,
  });
  setAccessToken(null);
}

export type AuthState =
  | { status: "loading" }
  | { status: "anon" }
  | { status: "authed" };

export const AuthContext = React.createContext<AuthState>({ status: "loading" });
export const useAuth = () => React.useContext(AuthContext);

export function canManageUsers(auth: AuthState): boolean {
  return auth.status === "authed";
}
