// AuthProvider.tsx
import React from "react";
import { useQuery } from "@tanstack/react-query";
import { apiUrl, setAccessToken } from "./api";
import { AuthContext, sessionFromAccessToken, type AuthState } from "./auth";
import type { AccessTokenResponse, AuthSession } from "./models/auth";

export function AuthProvider({ children }: Readonly<{ children: React.ReactNode }>) {
  const { data: session, isLoading, isSuccess } = useQuery<AuthSession>({
    queryKey: ["auth", "bootstrap"],
    queryFn: async () => {
      // call refresh to see if we have a session cookie
      const resp = await fetch(apiUrl("api/v1/auth/refresh"), { method: "POST", credentials: "include" });
      if (!resp.ok) throw new Error("not logged in");
      const data = (await resp.json()) as AccessTokenResponse;
      setAccessToken(data.access_token);
      const session = sessionFromAccessToken(data.access_token);
      if (!session) throw new Error("session missing from access token");
      return session;
    },
    retry: false,
  });

  const resolvedValue: AuthState =
    isLoading ? { status: "loading" } :
    isSuccess && session ? { status: "authed", ...session } :
    { status: "anon" };

  return <AuthContext.Provider value={resolvedValue}>{children}</AuthContext.Provider>;
}
