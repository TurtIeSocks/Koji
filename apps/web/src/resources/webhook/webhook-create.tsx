import { Create, SimpleForm } from "@/components/admin";
import { WebhookFormFields } from "./webhook-form";

export const WebhookCreate = () => (
  <Create>
    <SimpleForm>
      <WebhookFormFields />
    </SimpleForm>
  </Create>
);
