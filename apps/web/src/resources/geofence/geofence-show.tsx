import {
  TextField,
  ReferenceField,
  ReferenceArrayField,
  SingleFieldList,
  ChipField,
} from "@/components/admin";
import { ShowLive } from "@/components/realtime";
import { DeckGeoJsonField } from "@/components/deck";
import type { ShowProps } from "@/components/admin/views/show";

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
    </div>
  </ShowLive>
);
