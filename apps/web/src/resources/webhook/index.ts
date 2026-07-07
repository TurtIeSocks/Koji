import { Webhook } from "lucide-react";
import { WebhookList } from "./webhook-list";
import { WebhookCreate } from "./webhook-create";
import { WebhookEdit } from "./webhook-edit";
import { WebhookShow } from "./webhook-show";

export const webhook = {
  name: "webhook",
  list: WebhookList,
  create: WebhookCreate,
  edit: WebhookEdit,
  show: WebhookShow,
  recordRepresentation: "name",
  icon: Webhook,
};
