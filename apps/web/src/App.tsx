import { Admin, Resource, Layout } from "@/components/admin";
import { CustomRoutes } from "shadmin-core";
import { Route } from "react-router";
import { dataProvider } from "@/data-provider";
import { ImportWizard } from "@/resources/import/import-wizard";
import { authProvider } from "@/auth-provider";
import { geofence } from "@/resources/geofence";
import { route } from "@/resources/route";
import { project } from "@/resources/project";
import { property } from "@/resources/property";
import { tileserver } from "@/resources/tileserver";
import { plugins } from "@/resources/plugins";
import { Dashboard } from "@/dashboard/dashboard";
import { KojiAppBar } from "@/components/app-bar";
import { PasswordLoginPage } from "@/components/login/password-login-page";

const KojiLayout = (props: React.ComponentProps<typeof Layout>) => (
  <Layout {...props} appBar={KojiAppBar} />
);

function App() {
  return (
    <Admin
      dataProvider={dataProvider}
      authProvider={authProvider}
      layout={KojiLayout}
      dashboard={Dashboard}
      title="Kōji Admin"
      loginPage={PasswordLoginPage}
      disableTelemetry
    >
      <Resource {...geofence} group="Geo" />
      <Resource {...route} group="Geo" />
      <Resource {...project} group="Config" />
      <Resource {...property} group="Config" />
      <Resource {...tileserver} group="Config" />
      <Resource {...plugins} group="Config" />
      <CustomRoutes>
        <Route path="/import" element={<ImportWizard />} />
      </CustomRoutes>
    </Admin>
  );
}

export default App;
