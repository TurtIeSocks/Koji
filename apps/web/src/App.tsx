import { Admin, Resource, Layout } from "@/components/admin";
import { dataProvider } from "@/data-provider";
import { authProvider } from "@/auth-provider";
import { geofence } from "@/resources/geofence";
import { tileserver } from "@/resources/tileserver";
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
      <Resource {...tileserver} group="Config" />
    </Admin>
  );
}

export default App;
