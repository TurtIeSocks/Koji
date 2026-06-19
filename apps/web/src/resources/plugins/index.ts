import { Puzzle } from "lucide-react";
import { PluginsList } from "./plugins-list";
import { PluginsEdit } from "./plugins-edit";

export const plugins = {
  name: "plugins",
  list: PluginsList,
  edit: PluginsEdit,
  // No create/show — plugins are managed by the server, not user-created.
  recordRepresentation: "name",
  icon: Puzzle,
};
