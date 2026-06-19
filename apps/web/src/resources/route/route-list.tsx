import {
  DataTable,
  ReferenceField,
  FilterLiveSearch,
  FilterList,
  FilterListItem,
  BulkActionsToolbar,
  BulkDeleteButton,
} from "@/components/admin";
import { ListLive } from "@/components/realtime";
import { ROUTE_MODES } from "@/lib/constants";
import { BulkPublishButton } from "@/components/actions/publish-button";

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

export const RouteBulkToolbar = () => (
  <BulkActionsToolbar>
    <BulkPublishButton />
    <BulkDeleteButton />
  </BulkActionsToolbar>
);

export const RouteList = () => (
  <ListLive aside={<RouteFilters />}>
    <DataTable bulkActionsToolbar={<RouteBulkToolbar />}>
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
