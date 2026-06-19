import type { HTMLAttributes, ReactNode } from "react";
import { cn } from "@/lib/utils";
import { SidebarToggleButton } from "@/components/admin/buttons/sidebar-toggle-button";
import { TitlePortal } from "@/components/admin/layout/title-portal";
import { LocalesMenuButton } from "@/components/admin/buttons/locales-menu-button";
import { ThemeModeToggle } from "@/components/admin/layout/theme-mode-toggle";
import { RefreshButton } from "@/components/admin/buttons/refresh-button";
import { UserMenu } from "@/components/admin/layout/user-menu";

type AppBarProps = HTMLAttributes<HTMLElement> & {
  /**
   * When provided, replaces the entire default AppBar layout. The default is:
   * sidebar trigger, breadcrumb portal (with title portal inside), locales menu,
   * theme toggle, refresh button, user menu.
   */
  children?: ReactNode;
  /**
   * Replaces the default right-side action cluster (LocalesMenuButton +
   * ThemeModeToggle + RefreshButton). Pass a ReactNode to supply custom
   * actions, or omit to keep the default cluster.
   */
  toolbar?: ReactNode;
  /**
   * Replaces the default `<UserMenu />`. Pass `false` to suppress it entirely,
   * or a ReactNode to render a custom user menu.
   */
  userMenu?: ReactNode | false;
};

/**
 * The header at the top of the admin layout.
 *
 * Renders the default app bar with sidebar trigger, breadcrumb/title slot,
 * and toolbar buttons (locales, theme, refresh, user menu). Pass children to
 * fully replace the layout — useful when composing a custom header.
 *
 * Visually identical to the previous inline `<header>` in `<Layout>`; this
 * component simply extracts the markup for reuse in custom layouts.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/app-bar AppBar documentation}
 *
 * @example
 * import { AppBar, TitlePortal } from "@/components/admin";
 *
 * const MyAppBar = () => (
 *   <AppBar>
 *     <TitlePortal />
 *     <MyCustomButton />
 *   </AppBar>
 * );
 */
function AppBar({
  children,
  className,
  toolbar,
  userMenu,
  ...rest
}: AppBarProps) {
  return (
    <header
      className={cn(
        "flex h-16 md:h-12 shrink-0 items-center gap-2 px-4",
        className,
      )}
      {...rest}
    >
      {children ?? (
        <>
          <SidebarToggleButton />
          <div className="flex items-center min-w-0" id="breadcrumb" />
          <TitlePortal />
          {toolbar !== undefined ? (
            toolbar
          ) : (
            <>
              <LocalesMenuButton />
              <ThemeModeToggle />
              <RefreshButton />
            </>
          )}
          {userMenu !== false && (userMenu ?? <UserMenu />)}
        </>
      )}
    </header>
  );
}

export { AppBar, type AppBarProps };
