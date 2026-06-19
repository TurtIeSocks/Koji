import type { ComponentType, ErrorInfo, ReactNode } from "react";
import { Suspense, useState } from "react";
import { cn } from "@/lib/utils";
import type { CoreLayoutProps } from "shadmin-core";
import { ErrorBoundary } from "react-error-boundary";
import { SidebarProvider } from "@/components/ui/sidebar";
import { Notification } from "@/components/admin/feedback/notification";
import { AppSidebar } from "@/components/admin/layout/app-sidebar";
import { AppBar } from "@/components/admin/layout/app-bar";
import type { AppBarProps } from "@/components/admin/layout/app-bar";
import { Error } from "@/components/admin/feedback/error";
import type { ErrorProps } from "@/components/admin/feedback/error";
import { Loading } from "@/components/admin/feedback/loading";
import { Menu } from "@/components/admin/layout/menu";
import type { MenuProps } from "@/components/admin/layout/menu";
import { SkipNavigationButton } from "@/components/admin/buttons/skip-navigation-button";
import type { UnknownValue } from "@/lib/unknown-types";

interface LayoutProps extends CoreLayoutProps {
  /**
   * Replaces the default `<AppBar />` rendered at the top of the content area.
   */
  appBar?: ComponentType<AppBarProps>;
  /**
   * Replaces the default `<Menu />` rendered inside the sidebar content area.
   */
  menu?: ComponentType<MenuProps>;
  /**
   * Replaces the default `<AppSidebar>` component.
   */
  sidebar?: ComponentType<{ children?: ReactNode }>;
  /**
   * Replaces the default `<Error>` component rendered by the ErrorBoundary fallback.
   */
  error?: ComponentType<ErrorProps>;
}

/**
 * The main application layout with sidebar, header, and content area.
 *
 * Renders the app structure with a collapsible sidebar, header with breadcrumb navigation,
 * theme toggle, user menu, and main content area. Includes error boundary and loading states.
 *
 * Header markup is delegated to `<AppBar>`; sidebar markup to `<AppSidebar>`. The public
 * `<Layout>` API and visuals are unchanged.
 *
 * Accepts slot props (`appBar`, `menu`, `sidebar`, `error`) to replace individual sub-components
 * without re-implementing the full layout.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/layout Layout documentation}
 */
function Layout(props: LayoutProps) {
  const { appBar, menu, sidebar, error, ...rest } = props;
  const AppBarComponent = appBar ?? AppBar;
  const SidebarComponent = sidebar ?? AppSidebar;
  const ErrorComponent = error ?? Error;
  const MenuComponent = menu ?? Menu;
  const [errorInfo, setErrorInfo] = useState<ErrorInfo | undefined>(undefined);
  const handleError = (_: UnknownValue, info: ErrorInfo) => {
    setErrorInfo(info);
  };
  return (
    <SidebarProvider>
      <SidebarComponent>
        <MenuComponent />
      </SidebarComponent>
      <main
        className={cn(
          "ml-auto w-full max-w-full",
          "peer-data-[state=collapsed]:w-[calc(100%-var(--sidebar-width-icon)-1rem)]",
          "peer-data-[state=expanded]:w-[calc(100%-var(--sidebar-width))]",
          "sm:transition-[width] sm:duration-200 sm:ease-linear",
          "flex h-svh flex-col",
          "group-data-[scroll-locked=1]/body:h-full",
          "has-[main.fixed-main]:group-data-[scroll-locked=1]/body:h-svh",
        )}
      >
        <SkipNavigationButton />
        <AppBarComponent />
        <ErrorBoundary
          onError={handleError}
          fallbackRender={({ error, resetErrorBoundary }) => (
            <ErrorComponent
              error={error}
              errorInfo={errorInfo}
              resetErrorBoundary={resetErrorBoundary}
            />
          )}
        >
          <Suspense fallback={<Loading />}>
            <div className="flex flex-1 flex-col px-4 ">{rest.children}</div>
          </Suspense>
        </ErrorBoundary>
      </main>
      <Notification />
    </SidebarProvider>
  );
}

export { Layout, type LayoutProps };
