import { useState } from "react";
import { Controller, useForm } from "react-hook-form";
import { useNavigate, Navigate, Outlet, useLocation } from "react-router-dom";

import { InputText } from "primereact/inputtext";
import { classNames } from "primereact/utils";
import { Message } from "primereact/message";
import { Button } from "primereact/button";

import { HttpError } from "../api";
import type { LoginRequest } from "../models/auth";
import { Card } from "primereact/card";
import { login, useAuth  } from "../auth";

export function RequireAuth() {
  const auth = useAuth();
  const loc = useLocation();

  if (auth.status === "loading") return null; // or spinner
  if (auth.status === "anon") return <Navigate to="/login" replace state={{ from: loc }} />;

  return <Outlet />;
}


export default function Login() {
  const navigate = useNavigate();
  const [loginError, setLoginError] = useState<HttpError | null>(null);
  
  const {
    control,
    handleSubmit,
    formState: { errors, isSubmitting, isValid },
  } = useForm<LoginRequest>({ mode: 'onChange' });

  const submitter = async (data: LoginRequest) => {
    try {
      await login(data);
    } catch (err: unknown) {
      setLoginError(err as HttpError | null);
      return;
    }

    navigate('/');
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
