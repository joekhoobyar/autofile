import { useEffect, useState } from "react";
import { Controller, useForm } from "react-hook-form";
import { Link, useNavigate, Navigate, Outlet, useLocation } from "react-router-dom";
import { useQueryClient } from "@tanstack/react-query";

import { InputText } from "primereact/inputtext";
import { classNames } from "primereact/utils";
import { Message } from "primereact/message";
import { Button } from "primereact/button";
import { Skeleton } from "primereact/skeleton";

import { HttpError } from "../api";
import type { LoginRequest, RegisterRequest } from "../models/auth";
import { Card } from "primereact/card";
import { canAdminister, login, logout, register, useAuth } from "../auth";
import { usePublicSettings } from "../queries/useAppSettings";

export function RequireAuth() {
  const auth = useAuth();
  const loc = useLocation();

  if (auth.status === "loading") return null; // or spinner
  if (auth.status === "anon") return <Navigate to="/login" replace state={{ from: loc }} />;
  if (auth.forcePasswordChange && loc.pathname !== "/profile/password") {
    return <Navigate to="/profile/password" replace />;
  }

  return <Outlet />;
}

export function RequireAdmin() {
  const auth = useAuth();
  const loc = useLocation();

  if (auth.status === "loading") return null;
  if (auth.status === "anon") return <Navigate to="/login" replace state={{ from: loc }} />;
  if (auth.forcePasswordChange && loc.pathname !== "/profile/password") {
    return <Navigate to="/profile/password" replace />;
  }
  if (!canAdminister(auth)) return <Navigate to="/documents" replace />;

  return <Outlet />;
}


type LoginLocationState = {
  from?: { pathname?: string };
  signupSuccess?: boolean;
};

export default function Login() {
  const navigate = useNavigate();
  const location = useLocation();
  const queryClient = useQueryClient();
  const [loginError, setLoginError] = useState<HttpError | null>(null);
  const locationState = location.state as LoginLocationState | null;
  const from = locationState?.from?.pathname ?? "/";
  const { data: publicSettings } = usePublicSettings();
  
  const {
    control,
    handleSubmit,
    formState: { errors, isSubmitting, isValid },
  } = useForm<LoginRequest>({ mode: 'onChange' });

  const submitter = async (data: LoginRequest) => {
    try {
      const result = await login(data);
      queryClient.setQueryData(["auth", "bootstrap"], result);
      navigate(result.forcePasswordChange ? "/profile/password" : from, { replace: true });
    } catch (err: unknown) {
      setLoginError(err as HttpError | null);
      return;
    }
  };

  // PrimeReact-friendly error helper
  const errMsg = (name: keyof Partial<LoginRequest>) =>
    errors[name]?.message ? String(errors[name]?.message) : null;

  return (
    <Card title="Login">
      <form onSubmit={handleSubmit(submitter)}>
        <div className="p-fluid">

          {locationState?.signupSuccess && (
            <Message severity="success" text="Sign up successful. Next, an administrator will need to enable your account." />
          )}

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

          <div className="col-12 md:col-6 lg:col-4 flex align-items-center gap-3">
            <Button label="Login" type="submit" icon="pi pi-check" disabled={!isValid || isSubmitting} style={{ width: 'auto' }} />
            {publicSettings?.allow_user_registration === true && (
              <span>or <Link to="/signup">Sign up</Link></span>
            )}
          </div>
        </div>

      </form>
    </Card>
  );
}

type SignupForm = RegisterRequest & {
  confirmPassword: string;
};

export function Signup() {
  const navigate = useNavigate();
  const [registerError, setRegisterError] = useState<HttpError | null>(null);
  const { data: publicSettings, isLoading: publicSettingsLoading } = usePublicSettings();

  const {
    control,
    handleSubmit,
    formState: { errors, isSubmitting, isValid },
  } = useForm<SignupForm>({ mode: 'onChange' });

  const submitter = async (data: SignupForm) => {
    try {
      await register({
        username: data.username,
        email: data.email,
        display_name: data.display_name,
        password: data.password,
      });
      navigate("/login", { state: { signupSuccess: true } });
    } catch (err: unknown) {
      setRegisterError(err as HttpError | null);
      return;
    }
  };

  // PrimeReact-friendly error helper
  const errMsg = (name: keyof Partial<SignupForm>) =>
    errors[name]?.message ? String(errors[name]?.message) : null;

  if (publicSettingsLoading)
    return (
      <Card title="Sign Up">
        <div className="p-fluid">
          <div className="col-12 md:col-6 lg:col-4 flex flex-column gap-3">
            <Skeleton width="100%" height="2.5rem" />
            <Skeleton width="100%" height="2.5rem" />
            <Skeleton width="100%" height="2.5rem" />
            <Skeleton width="100%" height="2.5rem" />
            <Skeleton width="100%" height="2.5rem" />
            <Skeleton width="8rem" height="2.5rem" />
          </div>
        </div>
      </Card>
    );

  if (publicSettings && !publicSettings.allow_user_registration)
    return (
      <Card title="Sign Up">
        <Message severity="info" text="User registration is currently disabled." />
      </Card>
    );

  return (
    <Card title="Sign Up">
      <form onSubmit={handleSubmit(submitter)}>
        <div className="p-fluid">

          {registerError && (
            <Message severity="error" text={registerError.message} />
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

          {/* Email */}
          <div className="col-12 md:col-6 lg:col-4">
            <Controller name="email" control={control}
              rules={{
                required: 'Email is required',
                pattern: {
                  value: /^[^\s@]+@[^\s@]+\.[^\s@]+$/,
                  message: 'Valid email is required',
                },
              }}
              render={({ field }) => (
                <InputText id="email" {...field}
                  className={classNames({ 'p-invalid': !!errors.email })}
                  placeholder="Email" autoComplete="email"
                />
              )}
            />
            {errMsg('email') && <small className="p-error">{errMsg('email')}</small>}
          </div>

          {/* Display name */}
          <div className="col-12 md:col-6 lg:col-4">
            <Controller name="display_name" control={control}
              rules={{
                required: 'Display name is required',
              }}
              render={({ field }) => (
                <InputText id="display_name" {...field}
                  className={classNames({ 'p-invalid': !!errors.display_name })}
                  placeholder="Display name" autoComplete="name"
                />
              )}
            />
            {errMsg('display_name') && <small className="p-error">{errMsg('display_name')}</small>}
          </div>

          {/* Password */}
          <div className="col-12 md:col-6 lg:col-4">
            <Controller name="password" control={control}
              rules={{
                required: 'Password is required',
                minLength: {
                  value: 12,
                  message: 'Password must contain at least 12 characters',
                },
              }}
              render={({ field }) => (
                <InputText id="password" {...field} type="password"
                  className={classNames({ 'p-invalid': !!errors.password })}
                  placeholder="Password" autoComplete="new-password"
                />
              )}
            />
            {errMsg('password') && <small className="p-error">{errMsg('password')}</small>}
          </div>

          {/* Confirm password */}
          <div className="col-12 md:col-6 lg:col-4">
            <Controller name="confirmPassword" control={control}
              rules={{
                required: 'Password confirmation is required',
                validate: (value, formValues) => value === formValues.password || 'Passwords do not match',
              }}
              render={({ field }) => (
                <InputText id="confirmPassword" {...field} type="password"
                  className={classNames({ 'p-invalid': !!errors.confirmPassword })}
                  placeholder="Confirm password" autoComplete="new-password"
                />
              )}
            />
            {errMsg('confirmPassword') && <small className="p-error">{errMsg('confirmPassword')}</small>}
          </div>

          <div className="col-12 md:col-6 lg:col-4 flex align-items-center gap-3">
            <Button label="Sign Up" type="submit" icon="pi pi-check" disabled={!isValid || isSubmitting} style={{ width: 'auto' }} />
            <span>or <Link to="/login">Login</Link></span>
          </div>
        </div>

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
