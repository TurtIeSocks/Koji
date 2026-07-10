import {
  Create,
  SimpleForm,
  TextInput,
  SelectInput,
  ReferenceInput,
} from "@/components/admin";
import { MultiPolygonInput } from "@/components/leaflet";
import { GEOFENCE_MODES, DEFAULT_TILE_URL } from "@/lib/constants";
import { required } from "ra-core";
import { GeofencePropertiesInput } from "@/resources/geofence/geofence-properties-input";

// Metadata fields shared by Create + Edit. The geometry input is page-specific
// and lives OUTSIDE this component: Create uses the Leaflet MultiPolygonInput
// (below); Edit uses the deck <GeofenceMap> — so the edit page has exactly one
// map, not both.
export const GeofenceFormFields = () => (
  <>
    <TextInput source="name" validate={required()} />
    <SelectInput source="mode" choices={[...GEOFENCE_MODES]} defaultValue="unset" />
    <ReferenceInput source="parent" reference="geofence" />
    <GeofencePropertiesInput />
  </>
);

export const GeofenceCreate = () => (
  <Create>
    <SimpleForm>
      <GeofenceFormFields />
      {/* Geofences are Polygon OR MultiPolygon. MultiPolygonInput is the ONLY
          vendored input that round-trips MultiPolygon losslessly AND hydrates a
          stored Polygon (geoman renders it, saves as a 1-polygon MultiPolygon).
          Do NOT "simplify" to PolygonInput (drops all but the first polygon =>
          data loss) or GeoJsonInput (wraps in GeometryCollection / keeps-most-
          recent => wrong type or truncation). */}
      <MultiPolygonInput source="geometry" tileUrl={DEFAULT_TILE_URL} height={400} />
    </SimpleForm>
  </Create>
);
