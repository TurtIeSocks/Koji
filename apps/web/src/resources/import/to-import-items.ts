import type { ImportItem, ImportKind, OnCollision } from "@/lib/import-api";

interface FormFeature {
  type?: string;
  geometry?: { type?: string } | unknown;
  name?: string;
  kind?: ImportKind;          // user override (Assign step); often undefined
  mode?: string;
  parent?: string | null;
  route_parent?: string | null;
  projects?: number[];
  on_collision?: OnCollision;
  // ...plus GeoJSON `properties`, ignored here
}

const ROUTE_GEOMS = new Set(["MultiPoint", "LineString", "MultiLineString"]);

/** Derive kind from geometry type when the user did not override it. */
function kindForGeometry(geomType: string | undefined): ImportKind {
  return geomType && ROUTE_GEOMS.has(geomType) ? "route" : "geofence";
}

/** Map the wizard's form features to the exact `POST /internal/import` wire shape.
 *  `kind` comes from the row's override when set, else from geometry type
 *  (B6's `kind` SelectInput does not write RHF until the user interacts, so most
 *  rows arrive with `kind` undefined — derive it). */
export function featuresToImportItems(features: FormFeature[]): ImportItem[] {
  return features.map((f) => {
    const geomType = (f.geometry as { type?: string } | undefined)?.type;
    const kind: ImportKind = f.kind ?? kindForGeometry(geomType);
    const item: ImportItem = {
      kind,
      name: f.name ?? "",
      geometry: f.geometry,
      projects: (f.projects ?? []).map(Number),
      on_collision: f.on_collision ?? "skip",
    };
    if (f.mode) item.mode = f.mode;
    if (kind === "geofence" && f.parent != null) item.parent = f.parent;
    if (kind === "route" && f.route_parent != null) item.route_parent = f.route_parent;
    return item;
  });
}
