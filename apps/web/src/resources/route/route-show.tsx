import { TextField, ReferenceField } from "@/components/admin";
import { ShowLive } from "@/components/realtime";
import { DeckGeoJsonField } from "@/components/deck";
import type { ShowProps } from "@/components/admin/views/show";

export const RouteShow = (props: Pick<ShowProps, "id">) => (
  <ShowLive {...props}>
    <div className="flex flex-col gap-4 p-4">
      <div className="flex flex-col gap-2">
        <TextField source="name" />
        <TextField source="mode" />
        <TextField source="description" />
        <ReferenceField source="geofence_id" reference="geofence" empty="—" />
      </div>
      <DeckGeoJsonField source="geometry" variant="route" height={400} />
    </div>
  </ShowLive>
);
