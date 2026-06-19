import { Settings } from "lucide-react";
import { PropertyList } from "./property-list";
import { PropertyEdit } from "./property-edit";
import { PropertyCreate } from "./property-create";
import { PropertyShow } from "./property-show";

export const property = {
  name: "property",
  list: PropertyList,
  edit: PropertyEdit,
  create: PropertyCreate,
  show: PropertyShow,
  recordRepresentation: "name",
  icon: Settings,
};
