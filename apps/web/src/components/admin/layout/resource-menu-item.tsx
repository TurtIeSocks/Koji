import { createElement } from "react";
import { Link, useMatch } from "react-router";
import {
  useBasename,
  useCanAccess,
  useCreatePath,
  useGetResourceLabel,
  useResourceDefinitions,
} from "shadmin-core";
import type { ReactNode } from "react";
import { List } from "lucide-react";
import {
  SidebarMenuButton,
  SidebarMenuItem,
  useSidebar,
} from "@/components/ui/sidebar";
import { Skeleton } from "@/components/ui/skeleton";

type ResourceMenuItemProps = {
  /**
   * The resource name (matches the `name` of a `<Resource>` registered in `<Admin>`).
   * Used to look up the resource definition, label, icon and list path.
   */
  name: string;
  /**
   * Override the label. Defaults to the pluralised resource label.
   */
  primaryText?: ReactNode;
  /**
   * Override the icon. Defaults to the resource icon or `<List />`.
   */
  leftIcon?: ReactNode;
  /**
   * Extra CSS class appended to the underlying menu button.
   */
  className?: string;
  /**
   * Click handler invoked after the default navigation. The default
   * `<AppSidebar>` uses this to close the mobile drawer.
   */
  onClick?: () => void;
  /** Additional props forwarded to the underlying element (dense, tooltipProps, etc.). */
  [rest: string]: unknown;
};

/**
 * Sidebar entry linking to a resource list view.
 *
 * Resolves the resource definition (label, icon, list path) and renders a
 * `<SidebarMenuItem>` that navigates to the list view. Returns `null` when
 * the current user does not have `list` access via `useCanAccess`. Shows a
 * `<Skeleton>` while the permission check is pending.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/resource-menu-item ResourceMenuItem documentation}
 *
 * @example
 * import { ResourceMenuItem } from "@/components/admin";
 *
 * const CustomSidebarMenu = () => (
 *   <nav>
 *     <ResourceMenuItem name="posts" />
 *     <ResourceMenuItem name="comments" />
 *   </nav>
 * );
 */
function ResourceMenuItem({
  name,
  primaryText,
  leftIcon,
  className,
  onClick,
  ...rest
}: ResourceMenuItemProps) {
  const { canAccess, isPending } = useCanAccess({
    resource: name,
    action: "list",
  });
  const resources = useResourceDefinitions();
  const getResourceLabel = useGetResourceLabel();
  const createPath = useCreatePath();
  const basename = useBasename();
  const to = createPath({ resource: name, type: "list" });
  const match = useMatch({ path: to, end: to === `${basename}/` });
  const { openMobile, setOpenMobile } = useSidebar();

  if (isPending) {
    return <Skeleton className="h-8 w-full" />;
  }

  if (!resources?.[name] || !canAccess) return null;

  const handleClick = () => {
    if (openMobile) {
      setOpenMobile(false);
    }
    onClick?.();
  };

  const icon =
    leftIcon !== undefined ? (
      leftIcon
    ) : resources[name].icon ? (
      createElement(resources[name].icon)
    ) : (
      <List />
    );

  const label =
    primaryText !== undefined ? primaryText : getResourceLabel(name, 2);

  return (
    <SidebarMenuItem>
      <SidebarMenuButton
        asChild
        isActive={!!match}
        className={className}
        {...rest}
      >
        <Link to={to} state={{ _scrollToTop: true }} onClick={handleClick}>
          {icon}
          {label}
        </Link>
      </SidebarMenuButton>
    </SidebarMenuItem>
  );
}

export { ResourceMenuItem, type ResourceMenuItemProps };
