import { Route } from 'react-router'
import { CustomRoutes } from 'shadmin-core'
import { authProvider } from '@/auth-provider'
import { Admin, Resource } from '@/components/admin'
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

function App() {
  return (
    <Admin
      authProvider={authProvider}
      dashboard={Dashboard}
      dataProvider={dataProvider}
      disableTelemetry
      loginPage={PasswordLoginPage}
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
    </Admin>
  )
}

export default App
