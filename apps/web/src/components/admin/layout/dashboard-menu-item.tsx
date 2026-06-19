import type { ReactNode } from "react";
import { House } from "lucide-react";
import { useBasename, useTranslate } from "shadmin-core";
import { MenuItemLink } from "@/components/admin/layout/menu-item-link";

const defaultLeftIcon = <House />;

type DashboardMenuItemProps = {
  /**
   * Extra CSS class appended to the underlying menu button.
   */
  className?: string;
  /**
   * Click handler invoked after the default navigation. Useful to close the
   * mobile drawer or trigger side effects.
   */
  onClick?: () => void;
  /**
   * Replaces the default `<House />` icon. Pass any ReactNode.
   */
  leftIcon?: ReactNode;
  /**
   * Replaces the default translated label (`ra.page.dashboard`). Strings are
   * passed through `useTranslate` with the string itself as fallback; ReactNodes
   * are rendered as-is.
   */
  primaryText?: ReactNode;
};

/**
 * Sidebar entry linking to the dashboard (`/`).
 *
 * Renders only the dashboard link with the default House icon and a
 * translated label (`ra.page.dashboard`). The default `<AppSidebar>` uses
 * this component when an `<Admin dashboard>` is configured.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/dashboard-menu-item DashboardMenuItem documentation}
 *
 * @example
 * import { DashboardMenuItem } from "@/components/admin";
 *
 * const CustomMenu = () => (
 *   <nav>
 *     <DashboardMenuItem />
 *     ...
 *   </nav>
 * );
 */
function DashboardMenuItem({
  className,
  onClick,
  leftIcon,
  primaryText,
}: DashboardMenuItemProps) {
  const translate = useTranslate();
  const basename = useBasename();
  const finalText =
    primaryText === undefined
      ? translate("ra.page.dashboard", { _: "Dashboard" })
      : typeof primaryText === "string"
        ? translate(primaryText, { _: primaryText })
        : primaryText;
  return (
    <MenuItemLink
      to={`${basename}/`}
      primaryText={finalText}
      leftIcon={leftIcon ?? defaultLeftIcon}
      className={className}
      onClick={onClick}
    />
  );
}

export { DashboardMenuItem, type DashboardMenuItemProps };
