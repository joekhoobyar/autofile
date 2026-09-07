import React from "react";
import { apiMutate, setAccessToken } from "./api";
import type { AuthSession, LoginRequest, LoginResult } from "./models/auth";

export type { UserRole } from "./models/auth";

type JwtPayload = {
  uid?: unknown;
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

export function sessionFromAccessToken(token: string): AuthSession | null {
  const payload = decodeJwtPayload(token);
  if (payload?.role === "admin" || payload?.role === "user") {
    const userId = typeof payload.uid === "number" ? payload.uid : Number(payload.uid);
    if (Number.isInteger(userId)) {
      return { userId, role: payload.role };
    }
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
  const session = sessionFromAccessToken(data.access_token);
  if (!session) {
    throw new Error("Missing or invalid session in access token");
  }
  return session;
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
  | ({ status: "authed" } & AuthSession);

export const AuthContext = React.createContext<AuthState>({ status: "loading" });
export const useAuth = () => React.useContext(AuthContext);

export function canManageUsers(auth: AuthState): boolean {
  return auth.status === "authed" && auth.role === "admin";
}
