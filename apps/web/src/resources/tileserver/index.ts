import { Map } from "lucide-react";
import { TileserverList } from "./tileserver-list";
import { TileserverEdit } from "./tileserver-edit";
import { TileserverCreate } from "./tileserver-create";
import { TileserverShow } from "./tileserver-show";

export const tileserver = {
  name: "tileserver",
  list: TileserverList,
  edit: TileserverEdit,
  create: TileserverCreate,
  show: TileserverShow,
  recordRepresentation: "name",
  icon: Map,
};
