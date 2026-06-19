import { MapPin } from "lucide-react";
import { GeofenceList } from "./geofence-list";
import { GeofenceEdit } from "./geofence-edit";
import { GeofenceCreate } from "./geofence-create";
import { GeofenceShow } from "./geofence-show";

export const geofence = {
  name: "geofence",
  list: GeofenceList,
  edit: GeofenceEdit,
  create: GeofenceCreate,
  show: GeofenceShow,
  recordRepresentation: "name",
  icon: MapPin,
};
