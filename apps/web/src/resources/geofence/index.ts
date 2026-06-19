import { MapPin } from "lucide-react";
import { GeofenceList } from "./geofence-list";
import { GeofenceEdit } from "./geofence-edit";
import { GeofenceCreate } from "./geofence-create";

export const geofence = {
  name: "geofence",
  list: GeofenceList,
  edit: GeofenceEdit,
  create: GeofenceCreate,
  recordRepresentation: "name",
  icon: MapPin,
};
