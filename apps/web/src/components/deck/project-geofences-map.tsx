import { useMemo } from "react";
import { useGeofencesByIds } from "@/map/data/use-geo-features";
import { geofenceOverlayLayer, overlayTooltip } from "@/map/lib/geofence-overlay";
import { DeckMap } from "./deck-map";

/** Read-only map of a project's MEMBER geofences: fits camera to all of them,
 *  each fence hover-nameable and click-to-open-edit (via `geofenceOverlayLayer`
 *  / `overlayTooltip`). Rendered NORMAL style (not ghost) — the member fences
 *  are the page's content, not a dimmed backdrop. */
export function ProjectGeofencesMap({
  ids,
  height = 640,
}: {
  ids: (number | string)[];
  height?: number | string;
}) {
  const q = useGeofencesByIds(ids, ids.length > 0);
  const layers = useMemo(
    () => [geofenceOverlayLayer({ features: q.data?.features ?? [], ghost: false })],
    [q.data],
  );

  if (ids.length === 0) {
    return <ProjectGeofencesMapPlaceholder text="No geofences" height={height} />;
  }

  // `useGeofencesByIds` returns `data: undefined` on first render (and again
  // transiently whenever the id list changes, clearing the previous query's
  // cache entry). `DeckMap` latches its camera once at mount, so mounting it
  // with `fitBounds={null}` here would pin the camera at [0,0]/zoom 2 and
  // never re-fit once the real bounds arrive. Wait for data before mounting.
  if (!q.data) {
    return <ProjectGeofencesMapPlaceholder text="Loading…" height={height} />;
  }

  return (
    <DeckMap
      layers={layers}
      fitBounds={q.data}
      height={height}
      expandable
      getTooltip={overlayTooltip}
      controller={{ doubleClickZoom: true }}
    />
  );
}

function ProjectGeofencesMapPlaceholder({
  text,
  height,
}: {
  text: string;
  height: number | string;
}) {
  return (
    <div
      style={{ height, width: "100%" }}
      className="flex items-center justify-center rounded-md border bg-muted/30 text-sm text-muted-foreground"
      data-slot="project-geofences-map-empty"
    >
      {text}
    </div>
  );
}
