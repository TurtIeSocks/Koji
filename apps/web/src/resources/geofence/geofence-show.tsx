import {
  TextField,
  ReferenceField,
  ReferenceArrayField,
  ReferenceManyField,
  SingleFieldList,
  ChipField,
  DataTable,
  CreateButton,
} from "@/components/admin";
import { ShowLive } from "@/components/realtime";
import { DeckGeoJsonField } from "@/components/deck";
import type { ShowProps } from "@/components/admin/views/show";
import { useRecordContext } from "ra-core";

const NewRouteButton = () => {
  const record = useRecordContext();
  if (!record) return null;
  return (
    <CreateButton
      resource="route"
      label="New route"
      state={{ record: { geofence_id: record.id } }}
    />
  );
};

export const GeofenceShow = (props: Pick<ShowProps, "id">) => (
  <ShowLive {...props}>
    <div className="flex flex-col gap-4 p-4">
      <div className="flex flex-col gap-2">
        <TextField source="name" />
        <TextField source="mode" />
        <TextField source="geo_type" label="Geometry" />
        <ReferenceField source="parent" reference="geofence" empty="—" />
        <ReferenceArrayField source="projects" reference="project">
          <SingleFieldList>
            <ChipField source="name" />
          </SingleFieldList>
        </ReferenceArrayField>
      </div>
      <DeckGeoJsonField source="geometry" height={400} />
      <ReferenceManyField reference="route" target="geofence_id" label="Routes">
        <div className="flex flex-col gap-2">
          <div className="flex justify-end">
            <NewRouteButton />
          </div>
          <DataTable bulkActionButtons={false}>
            <DataTable.Col source="name" />
            <DataTable.Col source="mode" />
            <DataTable.Col source="points" label="Points" />
          </DataTable>
        </div>
      </ReferenceManyField>
    </div>
  </ShowLive>
);
