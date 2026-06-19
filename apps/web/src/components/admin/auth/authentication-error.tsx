import type { HTMLAttributes, ReactNode } from "react";
import { TriangleAlert } from "lucide-react";
import { useDefaultTitle } from "shadmin-core";

import { Title } from "@/components/admin/layout/title";
import { AccessDenied } from "@/components/admin/auth/access-denied";

interface AuthenticationErrorProps extends HTMLAttributes<HTMLDivElement> {
  className?: string;
  icon?: ReactNode;
  textPrimary?: string;
  textSecondary?: string;
}

/**
 * Full-page error screen displayed when authentication fails (for example
 * because of an expired session). Mirrors {@link AccessDenied} but uses a
 * warning triangle icon and different default texts.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/authentication-error AuthenticationError documentation}
 */
function AuthenticationError({
  icon = <TriangleAlert className="size-32" />,
  textPrimary = "ra.page.authentication_error",
  textSecondary = "ra.message.authentication_error",
  ...rest
}: AuthenticationErrorProps) {
  const title = useDefaultTitle();
  return (
    <>
      <Title defaultTitle={title} />
      <AccessDenied
        icon={icon}
        textPrimary={textPrimary}
        textSecondary={textSecondary}
        {...rest}
      />
    </>
  );
}

export { AuthenticationError, type AuthenticationErrorProps };
