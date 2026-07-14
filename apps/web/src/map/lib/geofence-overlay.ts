import { GeoJsonLayer } from "@deck.gl/layers";
import type { Layer, PickingInfo } from "@deck.gl/core";
import { EDITING_FILL, EDITING_LINE, GEOFENCE_FILL, GEOFENCE_LINE } from "@/map/lib/layers";
import { openGeofenceEdit } from "@/map/lib/open-geofence";

/** Read-only overlay of OTHER geofences — dimmed "ghost" or normal orange,
 *  each fence clickable (opens its edit page in a new tab) and hover-nameable.
 *  Shared by project member maps and geofence ghost-neighbor rendering. */
export function geofenceOverlayLayer(opts: {
  features: GeoJSON.Feature[];
  ghost: boolean;
  excludeId?: number | string | null;
  id?: string;
}): Layer {
  const { features, ghost, excludeId, id } = opts;
  const data =
    excludeId != null
      ? features.filter((f) => (f.id ?? (f.properties as { id?: number | string } | null)?.id) !== excludeId)
      : features;
  const fill = ghost ? EDITING_FILL : GEOFENCE_FILL;
  const line = ghost ? EDITING_LINE : GEOFENCE_LINE;

  return new GeoJsonLayer({
    id: id ?? "geofence-overlay",
    data,
    pickable: true,
    filled: true,
    stroked: true,
    getFillColor: fill,
    getLineColor: line,
    lineWidthMinPixels: 2,
    onClick: (info: PickingInfo) => {
      const clickedId = info.object?.id ?? info.object?.properties?.id;
      if (clickedId != null) openGeofenceEdit(clickedId);
    },
  });
}

/** Hover tooltip for the overlay: geofence name, or nothing for non-geofence /
 *  nameless picks. */
export function overlayTooltip(info: PickingInfo): { text: string } | null {
  const name = info.object?.properties?.name;
  return name != null ? { text: String(name) } : null;
}
