import type { ReactNode } from "react";
import { useHasDashboard, useResourceDefinitions } from "shadmin-core";
import {
  SidebarGroup,
  SidebarGroupContent,
  SidebarMenu,
} from "@/components/ui/sidebar";
import { cn } from "@/lib/utils";
import { DashboardMenuItem } from "@/components/admin/layout/dashboard-menu-item";
import { MenuItemLink } from "@/components/admin/layout/menu-item-link";
import type { ResourceDefinitionWithGroup } from "@/components/admin/resource";
import { ResourceMenuItemGroup } from "@/components/admin/layout/resource-menu-item-group";
import { ResourceMenuItem } from "@/components/admin/layout/resource-menu-item";

type MenuProps = {
  /**
   * When provided, replaces the default resource-list rendering. Use
   * `<Menu.Item>`, `<Menu.DashboardItem>` and `<Menu.ResourceItem>` as
   * children.
   */
  children?: ReactNode;
  /**
   * Extra CSS class appended to the wrapping `<SidebarMenu>`.
   */
  className?: string;
};

interface ResourceMenuGroup {
  label: string;
  resources: string[];
}

const getResourceMenuGroups = (
  resources: Record<string, ResourceDefinitionWithGroup>,
) => {
  const groupedResources: ResourceMenuGroup[] = [];
  const groupIndexes = new Map<string, number>();
  const ungroupedResources: string[] = [];

  for (const [name, resource] of Object.entries(resources)) {
    if (!resource.hasList) continue;

    const group = resource.group?.trim();

    if (!group) {
      ungroupedResources.push(name);
      continue;
    }

    const existingIndex = groupIndexes.get(group);

    if (existingIndex === undefined) {
      groupIndexes.set(group, groupedResources.length);
      groupedResources.push({ label: group, resources: [name] });
      continue;
    }

    groupedResources[existingIndex].resources.push(name);
  }

  return { groupedResources, ungroupedResources };
};

/**
 * Renders the sidebar menu: a dashboard link (if defined) followed by one
 * link per resource that exposes a list view. Resources can be grouped by
 * passing the same `group` string to multiple `<Resource>` declarations.
 *
 * `<Menu>` is the building block of the default `<AppSidebar>`. Pass
 * children to replace the auto-generated entries with a custom list.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/menu Menu documentation}
 *
 * @example // Use the default resource-based menu
 * import { Menu } from "@/components/admin";
 *
 * const Sidebar = () => <Menu />;
 *
 * @example // Define a custom menu
 * import { Settings, Book } from "lucide-react";
 * import { Menu } from "@/components/admin";
 *
 * const Sidebar = () => (
 *   <Menu>
 *     <Menu.DashboardItem />
 *     <Menu.ResourceItem name="posts" />
 *     <Menu.Item to="/settings" primaryText="Settings" leftIcon={<Settings />} />
 *     <Menu.Item to="/books" primaryText="Books" leftIcon={<Book />} />
 *   </Menu>
 * );
 */
function Menu({ children, className }: MenuProps) {
  const hasDashboard = useHasDashboard();
  const resources = useResourceDefinitions() as Record<
    string,
    ResourceDefinitionWithGroup
  >;
  const { groupedResources, ungroupedResources } =
    getResourceMenuGroups(resources);

  if (children !== undefined && children !== null) {
    return (
      <SidebarGroup>
        <SidebarGroupContent>
          <SidebarMenu className={cn(className)}>{children}</SidebarMenu>
        </SidebarGroupContent>
      </SidebarGroup>
    );
  }

  return (
    <>
      {hasDashboard ? (
        <SidebarGroup>
          <SidebarGroupContent>
            <SidebarMenu className={cn(className)}>
              <DashboardMenuItem />
            </SidebarMenu>
          </SidebarGroupContent>
        </SidebarGroup>
      ) : null}
      {groupedResources.map(({ label, resources }) => (
        <ResourceMenuItemGroup
          key={label}
          label={label}
          resources={resources}
          menuClassName={className}
        />
      ))}
      <ResourceMenuItemGroup
        resources={ungroupedResources}
        menuClassName={className}
      />
    </>
  );
}

// re-export menu pieces for convenience (mirrors the upstream react-admin API)
Menu.Item = MenuItemLink;
Menu.DashboardItem = DashboardMenuItem;
Menu.ResourceItem = ResourceMenuItem;
Menu.ResourceGroup = ResourceMenuItemGroup;

export { Menu, type MenuProps };
