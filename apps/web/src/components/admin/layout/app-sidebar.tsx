import type { CSSProperties, ReactNode } from "react";
import { Link } from "react-router";
import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
} from "@/components/ui/sidebar";
import { Shell } from "lucide-react";
import { Menu } from "@/components/admin/layout/menu";

interface AppSidebarProps {
  /**
   * Replaces the default `<Menu />` rendered inside the sidebar content area.
   * Pass a custom navigation tree when you need full control over sidebar items.
   */
  children?: ReactNode;
  /**
   * Width of the sidebar in its open/expanded state, in pixels. Defaults to 240.
   * Applied as an inline `style` override on the underlying `<Sidebar>` element.
   */
  size?: number;
  /**
   * Width of the sidebar in its collapsed/icon-only state, in pixels. Defaults to 55.
   * Applied as an inline `style` override on the underlying `<Sidebar>` element.
   */
  closedSize?: number;
  /**
   * When `true`, signals that the AppBar is always visible (not hidden on scroll).
   * Descendants can read this flag via context or props to adjust their top spacing.
   * The flag is accepted here so it can be forwarded to the sidebar tree without
   * requiring a separate context provider in the layout.
   */
  appBarAlwaysOn?: boolean;
}

/**
 * Navigation sidebar displaying menu items, allowing users to navigate between different sections of the application.
 *
 * The sidebar can collapse to an icon-only view and renders as a collapsible drawer on mobile devices.
 * It automatically includes links to the dashboard (if defined) and all list views defined in Resource components.
 * Menu rendering is delegated to `<Menu>` so each item can be reused individually.
 *
 * Included in the default Layout component.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/app-sidebar AppSidebar documentation}
 * @see {@link https://ui.shadcn.com/docs/components/sidebar shadcn/ui Sidebar component}
 * @see layout.tsx
 */
function AppSidebar({
  children,
  size = 240,
  closedSize = 55,
  appBarAlwaysOn: _appBarAlwaysOn,
}: AppSidebarProps) {
  return (
    <Sidebar
      variant="floating"
      collapsible="icon"
      style={
        {
          "--sidebar-width": `${size}px`,
          "--sidebar-width-icon": `${closedSize}px`,
        } as CSSProperties
      }
    >
      <SidebarHeader>
        <SidebarMenu>
          <SidebarMenuItem>
            <SidebarMenuButton
              asChild
              className="data-[slot=sidebar-menu-button]:p-1.5!"
            >
              <Link to="/">
                <Shell className="size-5!" />
                <span className="text-base font-semibold">Acme Inc.</span>
              </Link>
            </SidebarMenuButton>
          </SidebarMenuItem>
        </SidebarMenu>
      </SidebarHeader>
      <SidebarContent>{children ?? <Menu />}</SidebarContent>
      <SidebarFooter />
    </Sidebar>
  );
}

export { AppSidebar, type AppSidebarProps };
