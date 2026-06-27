import { Route } from 'react-router'
import { Authenticated, CustomRoutes } from 'shadmin-core'
import { MapIcon } from 'lucide-react'
import { authProvider } from '@/auth-provider'
import { Admin, Layout, Menu, Resource } from '@/components/admin'
import { PasswordLoginPage } from '@/components/login/password-login-page'
import { Dashboard } from '@/dashboard/dashboard'
import { dataProvider } from '@/data-provider'
import { geofence } from '@/resources/geofence'
import { ImportWizard } from '@/resources/import/import-wizard'
import { MapRoute } from '@/map/map-route'
import { plugins } from '@/resources/plugins'
import { project } from '@/resources/project'
import { property } from '@/resources/property'
import { route } from '@/resources/route'
import { tileserver } from '@/resources/tileserver'
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
      <CustomRoutes>
        <Route element={<ImportWizard />} path="/import" />
      </CustomRoutes>
      {/* Map is full-bleed (no admin sidebar/appbar chrome) — its own viewport.
          noLayout routes are PUBLIC by default (requireAuth doesn't cover them),
          so gate it explicitly with <Authenticated> → redirect to login if not. */}
      <CustomRoutes noLayout>
        <Route
          element={
            <Authenticated>
              <MapRoute />
            </Authenticated>
          }
          path="/map"
        />
      </CustomRoutes>
    </Admin>
  )
}

export default App
