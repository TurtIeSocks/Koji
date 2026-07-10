import { SimpleForm } from "@/components/admin";
import { EditLive } from "@/components/realtime";
import { RouteFormFields } from "./route-create";
import type { EditProps } from "@/components/admin/views/edit";

export const RouteEdit = (props: Pick<EditProps, "id">) => (
  <EditLive {...props}>
    {/* Wider than the default max-w-lg — the calc workbench map needs the room. */}
    <SimpleForm className="max-w-4xl">
      <RouteFormFields />
    </SimpleForm>
  </EditLive>
);
