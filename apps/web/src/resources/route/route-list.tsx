import {
  DataTable,
  ReferenceField,
  FilterLiveSearch,
  FilterList,
  FilterListItem,
  BulkActionsToolbar,
  BulkDeleteButton,
  CreateButton,
  ExportButton,
} from "@/components/admin";
import { ListLive } from "@/components/realtime";
import { ROUTE_MODES } from "@/lib/constants";
import { BulkPublishButton } from "@/components/actions/publish-button";
import { ImportButton } from "@/resources/import/import-button";

const RouteListActions = () => (
  <div className="flex items-center gap-2">
    <CreateButton />
    <ExportButton />
    <ImportButton />
  </div>
);

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
  <ListLive aside={<RouteFilters />} actions={<RouteListActions />}>
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
