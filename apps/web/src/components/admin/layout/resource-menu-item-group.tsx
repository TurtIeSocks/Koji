import { useResourceDefinitions } from "shadmin-core";
import { ChevronRightIcon } from "lucide-react";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";
import {
  SidebarGroup,
  SidebarGroupContent,
  SidebarGroupLabel,
  SidebarMenu,
} from "@/components/ui/sidebar";
import { cn } from "@/lib/utils";
import { ResourceMenuItem } from "@/components/admin/layout/resource-menu-item";

interface ResourceMenuItemGroupProps {
  /**
   * Optional section label. Omit it for the ungrouped resource section.
   */
  label?: string;
  /**
   * Resource names to render in this group. When omitted, all registered
   * resources with `hasList === true` are rendered automatically.
   */
  resources?: string[];
  /**
   * Extra CSS class appended to the wrapping sidebar group.
   */
  className?: string;
  /**
   * Extra CSS class appended to the underlying sidebar menu.
   */
  menuClassName?: string;
  /**
   * Initial open state for labeled groups.
   */
  defaultOpen?: boolean;
}

/**
 * Sidebar group containing resource menu entries.
 *
 * Renders a shadcn/ui `<SidebarGroup>` with an optional label and one
 * `<ResourceMenuItem>` per resource name. Labeled groups are collapsible by
 * default; unlabeled groups render as a plain section.
 *
 * @see {@link https://ui.shadcn.com/docs/components/radix/sidebar#sidebargroup shadcn/ui SidebarGroup}
 *
 * @example
 * import { ResourceMenuItemGroup } from "@/components/admin";
 *
 * const CustomSidebarMenu = () => (
 *   <ResourceMenuItemGroup label="Content" resources={["posts", "comments"]} />
 * );
 */
function ResourceMenuItemGroup({
  label,
  resources: resourcesProp,
  className,
  menuClassName,
  defaultOpen = true,
}: ResourceMenuItemGroupProps) {
  const resourceDefinitions = useResourceDefinitions();
  const resources =
    resourcesProp ??
    Object.keys(resourceDefinitions).filter(
      (name) => resourceDefinitions[name].hasList,
    );

  if (resources.length === 0) return null;

  const menu = (
    <SidebarGroupContent>
      <SidebarMenu className={cn(menuClassName)}>
        {resources.map((name) => (
          <ResourceMenuItem key={name} name={name} />
        ))}
      </SidebarMenu>
    </SidebarGroupContent>
  );

  if (!label) {
    return <SidebarGroup className={className}>{menu}</SidebarGroup>;
  }

  return (
    <Collapsible defaultOpen={defaultOpen} className="group/collapsible">
      <SidebarGroup className={className}>
        <SidebarGroupLabel asChild>
          <CollapsibleTrigger>
            {label}
            <ChevronRightIcon className="ml-auto transition-transform group-data-[state=open]/collapsible:rotate-90" />
          </CollapsibleTrigger>
        </SidebarGroupLabel>
        <CollapsibleContent>{menu}</CollapsibleContent>
      </SidebarGroup>
    </Collapsible>
  );
}

export { ResourceMenuItemGroup, type ResourceMenuItemGroupProps };
