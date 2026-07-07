import { TextField, BooleanField, ReferenceField } from "@/components/admin";
import { ShowLive } from "@/components/realtime";
import type { ShowProps } from "@/components/admin/views/show";
import { useRecordContext } from "ra-core";
import { WebhookTestButton } from "./webhook-test-button";

const ProjectOrGlobal = () => {
  const record = useRecordContext();
  if (record?.project_id == null) return <span>Global</span>;
  return (
    <ReferenceField source="project_id" reference="project">
      <TextField source="name" />
    </ReferenceField>
  );
};

export const WebhookShow = (props: Pick<ShowProps, "id">) => (
  <ShowLive {...props}>
    <div className="flex flex-col gap-4 p-4">
      <TextField source="name" />
      <TextField source="url" />
      <TextField source="mode" />
      <TextField source="method" />
      <BooleanField source="active" />
      <ProjectOrGlobal />
      <WebhookTestButton />
    </div>
  </ShowLive>
);
