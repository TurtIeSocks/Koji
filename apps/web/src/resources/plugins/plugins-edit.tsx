import { SimpleForm, TextInput, BooleanInput } from "@/components/admin";
import type { EditProps } from "@/components/admin/views/edit";
import { EditLive } from "@/components/realtime";
import { MonacoJsonInput } from "@/components/monaco";

export const PluginsEdit = (props: Pick<EditProps, "id">) => (
  <EditLive {...props}>
    <SimpleForm>
      {/* Read-only metadata — disabled to signal non-editable */}
      <TextInput source="name" disabled />
      <TextInput source="kind" disabled />
      <TextInput source="version" disabled />
      <TextInput source="entrypoint" disabled />
      <TextInput source="interpreter" disabled />
      <TextInput source="protocol" disabled />
      {/* Editable fields */}
      <BooleanInput source="enabled" />
      <TextInput source="description" />
      <MonacoJsonInput
        source="args_default"
        label="Default Arguments (JSON)"
        height={300}
      />
    </SimpleForm>
  </EditLive>
);
