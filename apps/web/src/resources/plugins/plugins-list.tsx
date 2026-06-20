import { DataTable, BooleanField, FilterLiveSearch } from "@/components/admin";
import { ListLive } from "@/components/realtime";

// Plugins are disk-owned + read-only declarations — the backend exposes no
// create endpoint (only enable/configure the overlay). So instead of a dead
// "Create" button, the empty state tells the user how a plugin actually appears.
const PluginsEmpty = () => (
  <div className="flex flex-col items-center gap-2 rounded-md border border-dashed p-8 text-center">
    <p className="text-sm font-medium">No plugins found</p>
    <p className="max-w-md text-sm text-muted-foreground">
      Plugins are managed on the server, not created here — drop a{" "}
      <code className="rounded bg-muted px-1 py-0.5">plugin.toml</code> in the
      server's plugins directory and it shows up here to enable + configure.
    </p>
  </div>
);

export const PluginsList = () => (
  <ListLive
    empty={<PluginsEmpty />}
    aside={
      <div className="flex w-56 flex-col gap-4">
        <FilterLiveSearch source="q" />
      </div>
    }
  >
    <DataTable>
      <DataTable.Col source="name" />
      <DataTable.Col source="kind" />
      <DataTable.Col source="enabled" label="Enabled">
        <BooleanField source="enabled" />
      </DataTable.Col>
      <DataTable.Col source="version" />
    </DataTable>
  </ListLive>
);
