import { useState } from "react";
import {
  Form,
  required,
  useLogin,
  useNotify,
  useTranslate,
} from "shadmin-core";
import type { SubmitHandler, FieldValues } from "react-hook-form";

import { cn } from "@/lib/utils";
import { notifyAuthError } from "@/lib/notify-auth-error";
import { Button } from "@/components/ui/button";
import { Spinner } from "@/components/ui/spinner";
import { TextInput } from "@/components/admin/inputs/text-input";

interface LoginFormProps {
  /**
   * Path the user is redirected to after a successful sign in.
   * Defaults to ra-core's default redirect (`/`).
   */
  redirectTo?: string;
  /**
   * Extra class names applied to the `<Form>` element.
   */
  className?: string;
}

/**
 * Email + password login form used by `<LoginPage>` and composable inside
 * any custom auth layout.
 *
 * Calls `authProvider.login()` via the `useLogin()` hook and surfaces
 * authentication errors through `useNotify()`. Returns the form on its own
 * — consumers are expected to provide the surrounding card / layout.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/login-form LoginForm documentation}
 *
 * @example
 * import { AuthLayout } from "@/components/admin/auth/auth-layout";
 * import { LoginForm } from "@/components/admin/auth/login-form";
 *
 * const MyLoginPage = () => (
 *   <AuthLayout title="Sign in">
 *     <LoginForm />
 *   </AuthLayout>
 * );
 */
function LoginForm(props: LoginFormProps) {
  const { redirectTo, className } = props;
  const [loading, setLoading] = useState(false);
  const login = useLogin();
  const notify = useNotify();
  const translate = useTranslate();

  const handleSubmit: SubmitHandler<FieldValues> = (values) => {
    setLoading(true);
    login(values, redirectTo)
      .then(() => {
        setLoading(false);
      })
      .catch((error) => {
        setLoading(false);
        notifyAuthError(notify, error);
      });
  };

  return (
    <Form
      className={cn("flex flex-col gap-8", className)}
      onSubmit={handleSubmit}
      mode="onChange"
      noValidate
    >
      <TextInput
        label="Email"
        source="email"
        type="email"
        autoComplete="email"
        autoFocus
        validate={required()}
      />
      <TextInput
        label="Password"
        source="password"
        type="password"
        autoComplete="current-password"
        validate={required()}
      />
      <Button type="submit" className="cursor-pointer" disabled={loading}>
        {loading ? <Spinner /> : null}
        {translate("ra.auth.sign_in", { _: "Sign in" })}
      </Button>
    </Form>
  );
}

export { LoginForm, type LoginFormProps };
