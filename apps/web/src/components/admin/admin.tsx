import {
  CoreAdminUI,
  type CoreAdminUIProps,
  CoreAdminContext,
  type CoreAdminContextProps,
  type CoreAdminProps,
  localStorageStore,
} from "shadmin-core";
import { i18nProvider as defaultI18nProvider } from "@/lib/i18n-provider";
import { Layout } from "@/components/admin/layout/layout";
import { LoginPage } from "@/components/admin/auth/login-page";
import { NotFound } from "@/components/admin/feedback/not-found";
import { Ready } from "@/components/admin/feedback/ready";
import { ThemeProvider } from "@/components/admin/layout/theme-provider";
import { AuthCallback } from "@/components/admin/auth/auth-callback";

/**
 * Props accepted by the `<Admin>` component on top of ra-core's `CoreAdminProps`.
 */
type AdminProps = CoreAdminProps;

/**
 * Props accepted by the {@link AdminContext} component. Identical to ra-core's
 * `CoreAdminContextProps` — re-exported so callers composing `<AdminContext>`
 * and `<AdminUI>` manually don't need to import from ra-core directly.
 */
type AdminContextProps = CoreAdminContextProps;

/**
 * Props accepted by the {@link AdminUI} component.
 */
type AdminUIProps = CoreAdminUIProps;

const defaultStore = localStorageStore();

/**
 * Provider half of `<Admin>`.
 *
 * Wraps `CoreAdminContext` and applies shadmin's default `store`
 * and `i18nProvider`. Use this directly when you need to interleave a
 * context-providing wrapper (for example the `<CommandMenu>` palette from
 * `@/components/extras/command-menu`) between the data providers and the
 * routed UI:
 *
 * @example
 * import {
 *   AdminContext,
 *   AdminUI,
 *   Resource,
 * } from "@/components/admin";
 * import { CommandMenu } from "@/components/extras/command-menu";
 *
 * const App = () => (
 *   <AdminContext dataProvider={dataProvider}>
 *     <CommandMenu>
 *       <AdminUI>
 *         <Resource name="posts" />
 *       </AdminUI>
 *     </CommandMenu>
 *   </AdminContext>
 * );
 */
function AdminContext({
  i18nProvider = defaultI18nProvider,
  store = defaultStore,
  ...rest
}: AdminContextProps) {
  return (
    <CoreAdminContext i18nProvider={i18nProvider} store={store} {...rest} />
  );
}

/**
 * UI half of `<Admin>`.
 *
 * Wraps `CoreAdminUI` with the {@link ThemeProvider} and the shadmin
 * default pages (login, not-found, ready, auth-callback). ra-core telemetry is
 * force-disabled here so the kit never pings an external endpoint.
 *
 * Must be rendered inside an {@link AdminContext}.
 */
function AdminUI(props: AdminUIProps) {
  const {
    authCallbackPage = AuthCallback,
    catchAll = NotFound,
    layout = Layout,
    loginPage = LoginPage,
    ready = Ready,
    title = "Shadmin",
    ...rest
  } = props;

  return (
    <ThemeProvider>
      <CoreAdminUI
        authCallbackPage={authCallbackPage}
        catchAll={catchAll}
        layout={layout}
        loginPage={loginPage}
        ready={ready}
        title={title}
        {...rest}
        disableTelemetry // forced off: the kit never pings an external telemetry endpoint
      />
    </ThemeProvider>
  );
}

/**
 * Root component of a shadmin application.
 *
 * Creates context providers to allow its children to access the app configuration.
 * Renders the main routes and layout, and delegates content area rendering to Resource children.
 * Composes {@link AdminContext} and {@link AdminUI} to provide a complete admin interface.
 *
 * Reach for the lower-level {@link AdminContext} + {@link AdminUI} pair when
 * you need to inject a wrapping component (such as the cmd+K
 * `<CommandMenu>` palette from `@/components/extras/command-menu`) between
 * the providers and the routed UI.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/admin Admin documentation}
 *
 * @example
 * // Basic usage with dataProvider and Resources
 * import { Admin, Resource } from "@/components/admin";
 * import simpleRestProvider from 'ra-data-simple-rest';
 *
 * const App = () => (
 *   <Admin dataProvider={simpleRestProvider('http://path.to.my.api')}>
 *     <Resource name="posts" list={PostList} />
 *   </Admin>
 * );
 *
 * @example
 * // With authentication and i18n
 * <Admin
 *   dataProvider={dataProvider}
 *   authProvider={authProvider}
 *   i18nProvider={i18nProvider}
 * >
 *   <Resource name="posts" list={PostList} edit={PostEdit} />
 * </Admin>
 */
function Admin(props: AdminProps) {
  const {
    accessDenied,
    authCallbackPage,
    authenticationError,
    authProvider,
    basename,
    catchAll,
    children,
    dashboard,
    dataProvider,
    disableTelemetry,
    error,
    i18nProvider,
    layout,
    loading,
    loginPage,
    queryClient,
    ready,
    requireAuth,
    store,
    title,
  } = props;

  return (
    <AdminContext
      authProvider={authProvider}
      basename={basename}
      dataProvider={dataProvider}
      i18nProvider={i18nProvider}
      queryClient={queryClient}
      store={store}
    >
      <AdminUI
        accessDenied={accessDenied}
        authCallbackPage={authCallbackPage}
        authenticationError={authenticationError}
        catchAll={catchAll}
        dashboard={dashboard}
        disableTelemetry={disableTelemetry}
        error={error}
        layout={layout}
        loading={loading}
        loginPage={loginPage}
        ready={ready}
        requireAuth={requireAuth}
        title={title}
      >
        {children}
      </AdminUI>
    </AdminContext>
  );
}

export {
  Admin,
  AdminContext,
  AdminUI,
  type AdminProps,
  type AdminContextProps,
  type AdminUIProps,
};
