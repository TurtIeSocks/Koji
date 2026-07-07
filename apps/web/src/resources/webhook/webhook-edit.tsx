import { SimpleForm } from "@/components/admin";
import type { EditProps } from "@/components/admin/views/edit";
import { EditLive } from "@/components/realtime";
import { WebhookFormFields } from "./webhook-form";

export const WebhookEdit = (props: Pick<EditProps, "id">) => (
  <EditLive {...props}>
    <SimpleForm>
      <WebhookFormFields />
    </SimpleForm>
  </EditLive>
);
