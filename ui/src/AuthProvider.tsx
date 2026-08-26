// AuthProvider.tsx
import React from "react";
import { useQuery } from "@tanstack/react-query";
import { apiUrl, setAccessToken } from "./api";
import { AuthContext, roleFromAccessToken, type AuthState, type UserRole } from "./auth";

export function AuthProvider({ children }: Readonly<{ children: React.ReactNode }>) {
  const { data: role, isLoading, isSuccess } = useQuery<UserRole>({
    queryKey: ["auth", "bootstrap"],
    queryFn: async () => {
      // call refresh to see if we have a session cookie
      const resp = await fetch(apiUrl("api/v1/auth/refresh"), { method: "POST", credentials: "include" });
      if (!resp.ok) throw new Error("not logged in");
      const data = (await resp.json()) as { access_token: string };
      setAccessToken(data.access_token);
      const role = roleFromAccessToken(data.access_token);
      if (!role) throw new Error("role missing from access token");
      return role;
    },
    retry: false,
  });

  const resolvedValue: AuthState =
    isLoading ? { status: "loading" } :
    isSuccess && role ? { status: "authed", role } :
    { status: "anon" };

  return <AuthContext.Provider value={resolvedValue}>{children}</AuthContext.Provider>;
}
