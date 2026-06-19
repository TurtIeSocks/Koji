import { Route } from "lucide-react";
import { RouteList } from "./route-list";
import { RouteEdit } from "./route-edit";
import { RouteCreate } from "./route-create";
import { RouteShow } from "./route-show";

export const route = {
  name: "route",
  list: RouteList,
  edit: RouteEdit,
  create: RouteCreate,
  show: RouteShow,
  recordRepresentation: "name",
  icon: Route,
};
