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
import { GEOFENCE_MODES, GEOMETRY_TYPES } from "@/lib/constants";
import { BulkPublishButton } from "@/components/actions/publish-button";
import { AssignParentBulkButton } from "@/components/actions/assign-parent-bulk";
import { AssignProjectsBulkButton } from "@/components/actions/assign-projects-bulk";
import { ImportButton } from "@/resources/import/import-button";

const GeofenceListActions = () => (
  <div className="flex items-center gap-2">
    <CreateButton />
    <ExportButton />
    <ImportButton />
  </div>
);

const GeofenceFilters = () => (
  <div className="flex w-56 flex-col gap-4">
    <FilterLiveSearch source="q" />
    <FilterList label="Mode">
      {GEOFENCE_MODES.map((m) => (
        <FilterListItem key={m.id} label={m.name} value={{ mode: m.id }} />
      ))}
    </FilterList>
    <FilterList label="Geometry">
      {GEOMETRY_TYPES.map((g) => (
        <FilterListItem key={g.id} label={g.name} value={{ geotype: g.id }} />
      ))}
    </FilterList>
  </div>
);

export const GeofenceBulkToolbar = () => (
  <BulkActionsToolbar>
    <BulkPublishButton />
    <AssignParentBulkButton />
    <AssignProjectsBulkButton />
    <BulkDeleteButton />
  </BulkActionsToolbar>
);

export const GeofenceList = () => (
  <ListLive aside={<GeofenceFilters />} actions={<GeofenceListActions />}>
    <DataTable bulkActionsToolbar={<GeofenceBulkToolbar />}>
      <DataTable.Col source="name" />
      <DataTable.Col source="parent" label="Parent">
        <ReferenceField source="parent" reference="geofence" />
      </DataTable.Col>
      <DataTable.Col source="mode" />
      <DataTable.Col source="geo_type" label="Geometry" />
    </DataTable>
  </ListLive>
);
