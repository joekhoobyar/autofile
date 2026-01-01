// AuthProvider.tsx
import React from "react";
import { useQuery } from "@tanstack/react-query";
import { API_HOST, setAccessToken } from "./api";
import { AuthContext, type AuthState } from "./auth";

export function AuthProvider({ children }: { children: React.ReactNode }) {
  const { isLoading, isSuccess } = useQuery({
    queryKey: ["auth", "bootstrap"],
    queryFn: async () => {
      // call refresh to see if we have a session cookie
      const resp = await fetch(`${API_HOST}/auth/refresh`, { method: "POST", credentials: "include" });
      if (!resp.ok) throw new Error("not logged in");
      const data = await resp.json();
      setAccessToken(data.access_token);
      return true;
    },
    retry: false,
  });

  const value: AuthState =
    isLoading ? { status: "loading" } :
    isSuccess ? { status: "authed" } :
    { status: "anon" };

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}
