import { useEffect, type HTMLAttributes, type ReactNode } from "react";
import { useCheckAuth } from "shadmin-core";
import { useNavigate } from "react-router";
import { AuthLayout } from "@/components/admin/auth/auth-layout";
import { LoginForm } from "@/components/admin/auth/login-form";

/**
 * Login page displayed when authentication is enabled and the user is not
 * authenticated.
 *
 * Automatically shown when an unauthenticated user tries to access a
 * protected route. Composes `<AuthLayout>` (two-column gradient shell with a
 * notification toaster) and `<LoginForm>` (email + password form wired to
 * `authProvider.login()`).
 *
 * Already-authenticated users land on the dashboard (`/`) instead of seeing
 * the login form.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/login-page LoginPage documentation}
 * @see {@link https://shadmin.turtlesocks.dev/docs/security Security documentation}
 */
interface LoginPageProps extends HTMLAttributes<HTMLDivElement> {
  redirectTo?: string;
  children?: ReactNode;
}

function LoginPage(props: LoginPageProps) {
  const { redirectTo, children, ...rest } = props;
  const checkAuth = useCheckAuth();
  const navigate = useNavigate();

  // Redirect already-authenticated users away from the login page.
  useEffect(() => {
    checkAuth({}, false)
      .then(() => {
        navigate("/");
      })
      .catch(() => {
        // not authenticated, stay on the login page
      });
  }, [checkAuth, navigate]);

  return (
    <AuthLayout title="Sign in" {...rest}>
      {children ?? <LoginForm redirectTo={redirectTo} />}
    </AuthLayout>
  );
}

export { LoginPage, type LoginPageProps };
