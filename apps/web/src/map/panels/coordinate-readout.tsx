import { useMapViewStore } from "@/map/stores/map-view-store";

/** Leaf: subscribes ONLY to settled primitives (never liveViewState). */
export function CoordinateReadout() {
  const lng = useMapViewStore((s) => s.settledViewState.longitude);
  const lat = useMapViewStore((s) => s.settledViewState.latitude);
  const zoom = useMapViewStore((s) => s.settledViewState.zoom);
  return (
    <div className="absolute bottom-4 left-4 z-10 rounded bg-background/90 px-2 py-1 font-mono text-xs shadow">
      {lng.toFixed(4)}, {lat.toFixed(4)} · z{zoom.toFixed(1)}
    </div>
  );
}
