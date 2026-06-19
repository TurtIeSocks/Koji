import { DataTable, BooleanField, FilterLiveSearch } from "@/components/admin";
import { ListLive } from "@/components/realtime";

export const PluginsList = () => (
  <ListLive
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
