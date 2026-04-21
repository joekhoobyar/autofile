import { useEffect, useState } from "react";
import { Controller, useForm } from "react-hook-form";
import { useNavigate, Navigate, Outlet, useLocation } from "react-router-dom";
import { useQueryClient } from "@tanstack/react-query";

import { InputText } from "primereact/inputtext";
import { classNames } from "primereact/utils";
import { Message } from "primereact/message";
import { Button } from "primereact/button";

import { HttpError } from "../api";
import type { LoginRequest } from "../models/auth";
import { Card } from "primereact/card";
import { canManageUsers, login, logout, useAuth } from "../auth";

export function RequireAuth() {
  const auth = useAuth();
  const loc = useLocation();

  if (auth.status === "loading") return null; // or spinner
  if (auth.status === "anon") return <Navigate to="/login" replace state={{ from: loc }} />;

  return <Outlet />;
}

export function RequireAdmin() {
  const auth = useAuth();
  const loc = useLocation();

  if (auth.status === "loading") return null;
  if (auth.status === "anon") return <Navigate to="/login" replace state={{ from: loc }} />;
  if (!canManageUsers(auth)) return <Navigate to="/documents" replace />;

  return <Outlet />;
}


export default function Login() {
  const navigate = useNavigate();
  const location = useLocation();
  const queryClient = useQueryClient();
  const [loginError, setLoginError] = useState<HttpError | null>(null);
  const from = (location.state as { from?: { pathname?: string } } | null)?.from?.pathname ?? "/";
  
  const {
    control,
    handleSubmit,
    formState: { errors, isSubmitting, isValid },
  } = useForm<LoginRequest>({ mode: 'onChange' });

  const submitter = async (data: LoginRequest) => {
    try {
      const result = await login(data);
      queryClient.setQueryData(["auth", "bootstrap"], result.role);
    } catch (err: unknown) {
      setLoginError(err as HttpError | null);
      return;
    }
    navigate(from, { replace: true });
  };

  // PrimeReact-friendly error helper
  const errMsg = (name: keyof Partial<LoginRequest>) =>
    errors[name]?.message ? String(errors[name]?.message) : null;

  return (
    <Card title="Login">
      <form onSubmit={handleSubmit(submitter)}>
        <div className="p-fluid">

          {loginError && (
            <Message severity="error" text={loginError.message} />
          )}

          {/* Username */}
          <div className="col-12 md:col-6 lg:col-4">
            <Controller name="username" control={control}
              rules={{
                required: 'Username is required',
              }}
              render={({ field }) => (
                <InputText id="username" {...field}
                  className={classNames({ 'p-invalid': !!errors.username })}
                  placeholder="Username" autoComplete="username"
                />
              )}
            />
            {errMsg('username') && <small className="p-error">{errMsg('username')}</small>}
          </div>

          {/* Password */}
          <div className="col-12 md:col-6 lg:col-4">
            <Controller name="password" control={control}
              rules={{
                required: 'Password is required',
              }}
              render={({ field }) => (
                <InputText id="name" {...field} type="password"
                  className={classNames({ 'p-invalid': !!errors.password })}
                  placeholder="Password" autoComplete="password"
                />
              )}
            />
            {errMsg('password') && <small className="p-error">{errMsg('password')}</small>}
          </div>
        </div>

        <Button label="Login" type="submit" icon="pi pi-check" disabled={!isValid || isSubmitting} />

      </form>
    </Card>
  );
}

export function Logout() {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [logoutError, setLogoutError] = useState<HttpError | null>(null);

  useEffect(() => {
    let active = true;

    const doLogout = async () => {
      try {
        await logout();
        await queryClient.resetQueries({ queryKey: ["auth", "bootstrap"] });
        if (!active) return;
        navigate("/login", { replace: true });
      } catch (err: unknown) {
        if (!active) return;
        setLogoutError(err as HttpError | null);
      }
    };

    void doLogout();

    return () => {
      active = false;
    };
  }, [navigate, queryClient]);

  return (
    <Card title="Logout">
      {logoutError ? (
        <Message severity="error" text={logoutError.message} />
      ) : (
        <Message severity="info" text="Signing you out..." />
      )}
    </Card>
  );
}
