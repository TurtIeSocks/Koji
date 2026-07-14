import { useMemo } from "react";
import { useGeofencesByIds } from "@/map/data/use-geo-features";
import { geofenceOverlayLayer, overlayTooltip } from "@/map/lib/geofence-overlay";
import { DeckMap } from "./deck-map";

/** Read-only map of a project's MEMBER geofences: fits camera to all of them,
 *  each fence hover-nameable and click-to-open-edit (via `geofenceOverlayLayer`
 *  / `overlayTooltip`). Rendered NORMAL style (not ghost) — the member fences
 *  are the page's content, not a dimmed backdrop. */
export function ProjectGeofencesMap({ ids }: { ids: (number | string)[] }) {
  const q = useGeofencesByIds(ids, ids.length > 0);
  const layers = useMemo(
    () => [geofenceOverlayLayer({ features: q.data?.features ?? [], ghost: false })],
    [q.data],
  );

  if (ids.length === 0) {
    return (
      <div
        style={{ height: 640, width: "100%" }}
        className="flex items-center justify-center rounded-md border bg-muted/30 text-sm text-muted-foreground"
        data-slot="project-geofences-map-empty"
      >
        No geofences
      </div>
    );
  }

  return (
    <DeckMap
      layers={layers}
      fitBounds={q.data ?? null}
      height={640}
      expandable
      getTooltip={overlayTooltip}
      controller={{ doubleClickZoom: true }}
    />
  );
}
