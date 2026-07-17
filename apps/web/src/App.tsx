import { FileTextIcon, MapIcon } from 'lucide-react'
import { Route } from 'react-router'
import { Authenticated, CustomRoutes } from 'shadmin-core'
import { authProvider } from '@api'
import { Admin, Layout, Menu, Resource } from '@/components/admin'
import { MapPlayground } from '@/components/deck'
import { ApiDocs } from '@/components/docs/api-docs'
import { PasswordLoginPage } from '@/components/login/password-login-page'
import { Dashboard } from '@/dashboard/dashboard'
import { dataProvider } from '@/data-provider'
import { geofence } from '@/resources/geofence'
import { ImportWizard } from '@/resources/import/import-wizard'
import { plugins } from '@/resources/plugins'
import { project } from '@/resources/project'
import { property } from '@/resources/property'
import { route } from '@/resources/route'
import { tileserver } from '@/resources/tileserver'
import { webhook } from '@/resources/webhook'
import { SidebarGroup, SidebarGroupContent, SidebarMenu } from '@/components/ui/sidebar'

/** Sidebar menu: auto-resource groups + a Views group with the Map link. */
function KojiMenu() {
  return (
    <>
      <Menu />
      <SidebarGroup>
        <SidebarGroupContent>
          <SidebarMenu>
            <Menu.Item to="/map" primaryText="Map" leftIcon={<MapIcon className="size-4" />} />
            <Menu.Item
              to="/docs"
              primaryText="API Docs"
              leftIcon={<FileTextIcon className="size-4" />}
            />
          </SidebarMenu>
        </SidebarGroupContent>
      </SidebarGroup>
    </>
  )
}

/** Layout variant that injects the Koji-specific sidebar menu. */
function KojiLayout(props: Parameters<typeof Layout>[0]) {
  return <Layout {...props} menu={KojiMenu} />
}

function App() {
  return (
    <Admin
      authProvider={authProvider}
      dashboard={Dashboard}
      dataProvider={dataProvider}
      disableTelemetry
      layout={KojiLayout}
      loginPage={PasswordLoginPage}
      // Secure-by-default: gate EVERY route incl. noLayout custom routes (/map)
      // behind auth. Without this, noLayout CustomRoutes render publicly.
      requireAuth
      title="Kōji"
    >
      <Resource {...project} group="Config" />
      <Resource {...geofence} group="Geo" />
      <Resource {...route} group="Geo" />
      <Resource {...property} group="Config" />
      <Resource {...tileserver} group="Config" />
      <Resource {...plugins} group="Config" />
      <Resource {...webhook} group="Config" />
      <CustomRoutes>
        <Route element={<ImportWizard />} path="/import" />
      </CustomRoutes>
      {/* API docs render full-bleed (Scalar owns the whole viewport), so noLayout
          like /map. Same public-by-default caveat → gate with <Authenticated>. */}
      <CustomRoutes noLayout>
        <Route
          element={
            <Authenticated>
              <ApiDocs />
            </Authenticated>
          }
          path="/docs"
        />
      </CustomRoutes>
      {/* Map is full-bleed (no admin sidebar/appbar chrome) — its own viewport.
          noLayout routes are PUBLIC by default (requireAuth doesn't cover them),
          so gate it explicitly with <Authenticated> → redirect to login if not. */}
      <CustomRoutes noLayout>
        <Route
          element={
            <Authenticated>
              <MapPlayground />
            </Authenticated>
          }
          path="/map"
        />
      </CustomRoutes>
    </Admin>
  )
}

export default App
