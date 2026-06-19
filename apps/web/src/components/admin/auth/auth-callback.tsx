import { useHandleAuthCallback } from "shadmin-core";

import { AuthError } from "@/components/admin/auth/auth-error";
import { Loading } from "@/components/admin/feedback/loading";

/**
 * A standalone page to be used as a redirection target for external authentication services (e.g. OAuth).
 *
 * It displays a loading indicator while processing the authentication response,
 * then redirects to the admin app or shows an error message if authentication fails.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/auth-callback AuthCallback documentation}
 *
 * @example
 * import { Admin } from "@/components/admin";
 * import MyAuthCallbackPage from "@/components/admin/MyAuthCallbackPage";
 *
 * const App = () => (
 *   <Admin authCallbackPage={MyAuthCallbackPage} authProvider={authProvider}>
 *     ...
 *   </Admin>
 * );
 */
function AuthCallback() {
  const { error } = useHandleAuthCallback();
  if (error) {
    return (
      <AuthError
        message={(error as Error) ? (error as Error).message : undefined}
      />
    );
  }
  return <Loading />;
}

export { AuthCallback };
