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
import { geometryBounds } from "@/components/deck/bounds";
import { padBbox, useNeighborOverlay } from "@/components/deck/use-neighbor-overlay";
import { Button } from "@/components/ui/button";
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

// `useRecordContext` only resolves inside `<ShowLive>`'s provider — this tiny
// wrapper reads the fence's own mode/geometry to drive DeckGeoJsonField's
// mode-based marker toggle, mirroring RouteShowMap's record read.
const FenceMapField = () => {
  const record = useRecordContext();
  const nb = useNeighborOverlay(
    record?.geometry ? padBbox(geometryBounds(record.geometry as GeoJSON.Geometry), 0.2) : null,
    record?.id,
  );
  return (
    <DeckGeoJsonField
      source="geometry"
      markerMode={record?.mode}
      markerArea={record?.geometry as GeoJSON.Geometry | undefined}
      expandable
      extraLayers={nb.layers}
      getTooltip={nb.getTooltip}
      extraControls={
        <Button
          type="button"
          size="sm"
          variant="outline"
          className="bg-background/95"
          onClick={() => nb.setOn(!nb.on)}
        >
          {nb.on ? "Hide Neighbors" : "Show Neighbors"}
        </Button>
      }
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
      <FenceMapField />
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
