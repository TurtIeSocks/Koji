import { Admin, Resource, Layout } from "@/components/admin";
import { dataProvider } from "@/data-provider";
import { authProvider } from "@/auth-provider";
import { geofence } from "@/resources/geofence";
import { Dashboard } from "@/dashboard/dashboard";
import { KojiAppBar } from "@/components/app-bar";

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
      disableTelemetry
    >
      <Resource {...geofence} group="Geo" />
    </Admin>
  );
}

export default App;
