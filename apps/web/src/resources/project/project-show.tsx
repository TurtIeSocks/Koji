import {
  TextField,
  BooleanField,
  ReferenceArrayField,
  SingleFieldList,
  ChipField,
} from "@/components/admin";
import { ShowLive } from "@/components/realtime";
import type { ShowProps } from "@/components/admin/views/show";

export const ProjectShow = (props: Pick<ShowProps, "id">) => (
  <ShowLive {...props}>
    <div className="flex flex-col gap-4 p-4">
      <TextField source="name" />
      <BooleanField source="golbat" label="Golbat Sync" />
      <ReferenceArrayField source="geofences" reference="geofence">
        <SingleFieldList>
          <ChipField source="name" />
        </SingleFieldList>
      </ReferenceArrayField>
    </div>
  </ShowLive>
);
