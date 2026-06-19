import { Children, useCallback, useState, type ReactNode } from "react";
import {
  useAuthProvider,
  useGetIdentity,
  useTranslate,
  UserMenuContext,
} from "shadmin-core";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";

import { Logout } from "@/components/admin/auth/logout";

type UserMenuProps = {
  children?: ReactNode;
  label?: string;
  icon?: ReactNode;
};

/**
 * A user menu component displayed in the top right corner of the admin layout.
 *
 * Provides access to user-related actions such as profile, settings, and logout.
 * Displays the user's avatar and name from the identity provider, and includes a logout option.
 * Only displays in applications using authentication.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/user-menu UserMenu documentation}
 */
function UserMenu({ children, label, icon }: UserMenuProps) {
  const authProvider = useAuthProvider();
  const { isPending, data: identity } = useGetIdentity();
  const translate = useTranslate();

  const [open, setOpen] = useState(false);

  const handleToggleOpen = useCallback(() => {
    setOpen((prevOpen) => !prevOpen);
  }, []);

  const handleClose = useCallback(() => {
    setOpen(false);
  }, []);

  if (!authProvider) return null;

  if (isPending) {
    return <Skeleton className="size-8 rounded-full ml-2" />;
  }

  const ariaLabel =
    label != null
      ? translate(label, { _: label })
      : identity?.fullName ||
        translate("ra.auth.user_menu", { _: "User menu" });

  return (
    <UserMenuContext.Provider value={{ onClose: handleClose }}>
      <DropdownMenu open={open} onOpenChange={handleToggleOpen}>
        <DropdownMenuTrigger asChild>
          <Button
            variant="ghost"
            className="relative size-8 ml-2 rounded-full"
            aria-label={ariaLabel}
          >
            {icon ? (
              icon
            ) : (
              <Avatar className="size-8">
                <AvatarImage src={identity?.avatar} alt={identity?.fullName} />
                <AvatarFallback>{identity?.fullName?.charAt(0)}</AvatarFallback>
              </Avatar>
            )}
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent className="w-56" align="end" forceMount>
          <DropdownMenuLabel className="font-normal">
            <div className="flex flex-col gap-y-1">
              <p className="text-sm font-medium leading-none">
                {identity?.fullName}
              </p>
            </div>
          </DropdownMenuLabel>
          <DropdownMenuSeparator />
          {children}
          {Children.count(children) > 0 && <DropdownMenuSeparator />}
          <Logout />
        </DropdownMenuContent>
      </DropdownMenu>
    </UserMenuContext.Provider>
  );
}

export { UserMenu, type UserMenuProps };
