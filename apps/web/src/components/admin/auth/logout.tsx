import { useCallback, type ReactNode } from "react";
import { Translate, useAuthState, useLogout } from "shadmin-core";
import { LogOut } from "lucide-react";

import { DropdownMenuItem } from "@/components/ui/dropdown-menu";
import { cn } from "@/lib/utils";

interface LogoutProps {
  className?: string;
  /**
   * Path the user is redirected to after a successful logout. Defaults to
   * the authProvider's configured logout redirect.
   */
  redirectTo?: string;
  /**
   * Custom icon rendered alongside the logout label. Defaults to a power
   * icon.
   */
  icon?: ReactNode;
}

/**
 * Logout menu item, designed to be rendered as a child of
 * {@link UserMenu} (or any shadcn `<DropdownMenuContent>`).
 *
 * Calls `useLogout()` on click and renders nothing when the user is not
 * authenticated.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/logout Logout documentation}
 */
function Logout({ className, redirectTo, icon }: LogoutProps) {
  const { authenticated } = useAuthState();
  const logout = useLogout();

  const handleClick = useCallback(
    () => logout(null, redirectTo, false),
    [logout, redirectTo],
  );

  if (!authenticated) return null;

  return (
    <DropdownMenuItem
      onClick={handleClick}
      className={cn("cursor-pointer", className)}
    >
      {icon ?? <LogOut />}
      <Translate i18nKey="ra.auth.logout">Log out</Translate>
    </DropdownMenuItem>
  );
}

export { Logout, type LogoutProps };
