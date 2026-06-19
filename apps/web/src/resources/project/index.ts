import { FolderOpen } from "lucide-react";
import { ProjectList } from "./project-list";
import { ProjectEdit } from "./project-edit";
import { ProjectCreate } from "./project-create";
import { ProjectShow } from "./project-show";

export const project = {
  name: "project",
  list: ProjectList,
  edit: ProjectEdit,
  create: ProjectCreate,
  show: ProjectShow,
  recordRepresentation: "name",
  icon: FolderOpen,
};
