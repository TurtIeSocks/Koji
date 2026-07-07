import { Webhook } from "lucide-react";
import { WebhookList } from "./webhook-list";
import { WebhookCreate } from "./webhook-create";
import { WebhookEdit } from "./webhook-edit";

export const webhook = {
  name: "webhook",
  list: WebhookList,
  create: WebhookCreate,
  edit: WebhookEdit,
  recordRepresentation: "name",
  icon: Webhook,
};
