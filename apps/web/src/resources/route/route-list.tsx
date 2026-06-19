import {
  DataTable,
  ReferenceField,
  FilterLiveSearch,
  FilterList,
  FilterListItem,
} from "@/components/admin";
import { ListLive } from "@/components/realtime";
import { ROUTE_MODES } from "@/lib/constants";

const RouteFilters = () => (
  <div className="flex w-56 flex-col gap-4">
    <FilterLiveSearch source="q" />
    <FilterList label="Mode">
      {ROUTE_MODES.map((m) => (
        <FilterListItem key={m.id} label={m.name} value={{ mode: m.id }} />
      ))}
    </FilterList>
  </div>
);

export const RouteList = () => (
  <ListLive aside={<RouteFilters />}>
    <DataTable>
      <DataTable.Col source="name" />
      <DataTable.Col source="description" />
      <DataTable.Col source="mode" />
      <DataTable.Col source="geofence_id" label="Geofence">
        <ReferenceField source="geofence_id" reference="geofence" />
      </DataTable.Col>
      <DataTable.Col source="points" label="Points" />
    </DataTable>
  </ListLive>
);
