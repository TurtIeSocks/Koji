import { useState } from "react";
import {
  Form,
  required,
  useLogin,
  useNotify,
  useTranslate,
} from "shadmin-core";
import type { SubmitHandler, FieldValues } from "react-hook-form";
import { LogIn } from "lucide-react";

import { cn } from "@/lib/utils";
import { notifyAuthError } from "@/lib/notify-auth-error";
import { Button } from "@/components/ui/button";
import { Spinner } from "@/components/ui/spinner";
import { TextInput } from "@/components/admin/inputs/text-input";
import { PasswordInput } from "@/components/admin/inputs/password-input";

interface LoginWithEmailProps {
  /**
   * Custom submit handler. When provided, it is called with the form values
   * (`{ email, password }`) and is responsible for signing the user in.
   */
  onSubmit?: (values: {
    email: string;
    password: string;
  }) => Promise<void> | void;
  /**
   * Forces the submit button into the loading state. When `onSubmit` is not
   * provided, the component manages its own loading state internally.
   */
  loading?: boolean;
  /**
   * Path the user is redirected to after a successful sign in. Only used
   * when `onSubmit` is omitted (default `useLogin()` flow).
   */
  redirectTo?: string;
  /**
   * Label of the submit button. Defaults to the i18n key `ra.auth.sign_in`.
   */
  submitLabel?: string;
  /**
   * Extra class names applied to the `<Form>` element.
   */
  className?: string;
}

/**
 * Email + password login form.
 *
 * Mirrors `<LoginForm>` but uses `email` instead of `username` for the first
 * field. By default, submitting the form calls `useLogin()` with
 * `{ email, password }`. Providing the `onSubmit` prop overrides this behavior
 * so consumers can perform their own sign-in flow against a custom backend.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/login-with-email LoginWithEmail documentation}
 *
 * @example
 * import { AuthLayout } from "@/components/admin/auth/auth-layout";
 * import { LoginWithEmail } from "@/components/admin/auth/login-with-email";
 *
 * const SignInPage = () => (
 *   <AuthLayout title="Sign in" subtitle="Enter your email and password">
 *     <LoginWithEmail />
 *   </AuthLayout>
 * );
 */
function LoginWithEmail(props: LoginWithEmailProps) {
  const {
    onSubmit,
    loading: loadingProp,
    redirectTo,
    submitLabel,
    className,
  } = props;
  const [internalLoading, setInternalLoading] = useState(false);
  const login = useLogin();
  const notify = useNotify();
  const translate = useTranslate();
  const loading = loadingProp ?? internalLoading;

  const handleSubmit: SubmitHandler<FieldValues> = async (values) => {
    const formValues = values as { email: string; password: string };
    if (onSubmit) {
      try {
        await onSubmit(formValues);
      } catch (error) {
        notifyAuthError(notify, error);
      }
      return;
    }

    setInternalLoading(true);
    login(
      { email: formValues.email, password: formValues.password },
      redirectTo,
    )
      .then(() => {
        setInternalLoading(false);
      })
      .catch((error) => {
        setInternalLoading(false);
        notifyAuthError(notify, error);
      });
  };

  return (
    <Form
      mode="onChange"
      noValidate
      className={cn("flex flex-col gap-8", className)}
      onSubmit={handleSubmit}
    >
      <TextInput
        label={translate("ra.auth.email", { _: "Email" })}
        source="email"
        type="email"
        autoComplete="email"
        validate={required()}
      />
      <PasswordInput
        label={translate("ra.auth.password", { _: "Password" })}
        source="password"
        autoComplete="current-password"
        validate={required()}
      />
      <Button type="submit" className="cursor-pointer" disabled={loading}>
        {loading ? <Spinner /> : <LogIn />}
        {submitLabel ?? translate("ra.auth.sign_in", { _: "Sign in" })}
      </Button>
    </Form>
  );
}

export { LoginWithEmail, type LoginWithEmailProps };
