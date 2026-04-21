import React from "react";
import { apiMutate, setAccessToken } from "./api";
import type { LoginRequest, LoginResult, UserRole } from "./models/auth";

export type { UserRole } from "./models/auth";

type JwtPayload = {
  role?: unknown;
};

function decodeJwtPayload(token: string): JwtPayload | null {
  const parts = token.split(".");
  if (parts.length !== 3) {
    return null;
  }

  try {
    const base64Url = parts[1].replace(/-/g, "+").replace(/_/g, "/");
    const padding = "=".repeat((4 - (base64Url.length % 4)) % 4);
    const base64 = `${base64Url}${padding}`;
    const json = atob(base64);
    return JSON.parse(json) as JwtPayload;
  } catch {
    return null;
  }
}

export function roleFromAccessToken(token: string): UserRole | null {
  const payload = decodeJwtPayload(token);
  if (payload?.role === "admin" || payload?.role === "user") {
    return payload.role;
  }
  return null;
}

export async function login(user: LoginRequest): Promise<LoginResult> {
  const data = await apiMutate<{ access_token: string }>("api/v1/auth/login", {
    method: 'POST',
    body: user,
    retryOn401: false,
  });
  setAccessToken(data.access_token);
  const role = roleFromAccessToken(data.access_token);
  if (!role) {
    throw new Error("Missing or invalid role in access token");
  }
  return { role };
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
  | { status: "authed"; role: UserRole };

export const AuthContext = React.createContext<AuthState>({ status: "loading" });
export const useAuth = () => React.useContext(AuthContext);

export function canManageUsers(auth: AuthState): boolean {
  return auth.status === "authed" && auth.role === "admin";
}
