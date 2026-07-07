import {
  TextField,
  BooleanField,
  ReferenceArrayField,
  ReferenceManyField,
  SingleFieldList,
  ChipField,
  DataTable,
  CreateButton,
} from "@/components/admin";
import { ShowLive } from "@/components/realtime";
import type { ShowProps } from "@/components/admin/views/show";
import { useRecordContext } from "ra-core";

const AddWebhookButton = () => {
  const record = useRecordContext();
  if (!record) return null;
  return <CreateButton resource="webhook" label="Add webhook" />;
};

export const ProjectShow = (props: Pick<ShowProps, "id">) => (
  <ShowLive {...props}>
    <div className="flex flex-col gap-4 p-4">
      <TextField source="name" />
      <ReferenceArrayField source="geofences" reference="geofence">
        <SingleFieldList>
          <ChipField source="name" />
        </SingleFieldList>
      </ReferenceArrayField>
      <ReferenceManyField reference="webhook" target="project_id" label="Webhooks">
        <div className="flex flex-col gap-2">
          <div className="flex justify-end">
            <AddWebhookButton />
          </div>
          <DataTable bulkActionButtons={false}>
            <DataTable.Col source="name" />
            <DataTable.Col source="url" />
            <DataTable.Col source="mode" />
            <DataTable.Col source="active" label="Active">
              <BooleanField source="active" />
            </DataTable.Col>
          </DataTable>
        </div>
      </ReferenceManyField>
    </div>
  </ShowLive>
);
