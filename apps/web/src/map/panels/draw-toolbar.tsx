import { Button } from "@/components/ui/button";
import { useMapUIStore } from "@/map/stores/map-ui-store";
import { allGeometries } from "@/map/lib/edit-serialize";
import { mergeSelected } from "@/map/lib/merge-polygons";
import type { DrawMode } from "@/map/lib/edit-modes";
import { useDataProvider, useNotify } from "shadmin-core";

const MODES: { mode: DrawMode; label: string }[] = [
  { mode: "drawPolygon", label: "Polygon" },
  { mode: "drawRectangle", label: "Rectangle" },
  { mode: "modify", label: "Modify" },
  { mode: "translate", label: "Move" },
];

/** Leaf: subscribes to drawMode + the edit actions only (S3). */
export function DrawToolbar() {
  const drawMode = useMapUIStore((s) => s.drawMode);
  const draftFeatures = useMapUIStore((s) => s.draftFeatures);
  const setDrawMode = useMapUIStore((s) => s.setDrawMode);
  const setDraftFeatures = useMapUIStore((s) => s.setDraftFeatures);
  const setSelectedFeatureIndexes = useMapUIStore((s) => s.setSelectedFeatureIndexes);
  const editingGeofenceId = useMapUIStore((s) => s.editingGeofenceId);
  const clearDraft = useMapUIStore((s) => s.clearDraft);
  const dataProvider = useDataProvider();
  const notify = useNotify();

  const handleMerge = () => {
    // Merge ALL drawn polygons (single-click can only hold one selection, so
    // "Merge" combining everything is the intuitive behavior here).
    const allIdx = draftFeatures.features.map((_, i) => i);
    setDraftFeatures(mergeSelected(draftFeatures, allIdx));
    setSelectedFeatureIndexes([]);
  };

  const createGeometries = (geometries: GeoJSON.Geometry[]) => {
    const stamp = Date.now();
    return Promise.all(
      geometries.map((geometry, i) =>
        dataProvider.create("geofence", {
          data: { name: `map-${stamp}-${i + 1}`, mode: "Unset", geometry },
        }),
      ),
    );
  };

  const handleSave = async () => {
    const geometries = allGeometries(draftFeatures);
    if (geometries.length === 0) return;
    if (editingGeofenceId) {
      // Editing an EXISTING geofence → update it in place with the first shape.
      // Any ADDITIONAL shapes drawn during the edit session are saved as new
      // geofences too, not silently dropped.
      const [first, ...rest] = geometries;
      await dataProvider.update("geofence", {
        id: editingGeofenceId,
        data: { geometry: first },
        previousData: { id: editingGeofenceId },
      });
      if (rest.length > 0) await createGeometries(rest);
      clearDraft();
      notify(rest.length > 0 ? `Geofence updated (+${rest.length} new)` : "Geofence updated", { type: "info" });
      return;
    }
    // New shapes → one geofence per drawn feature.
    await createGeometries(geometries);
    clearDraft();
    notify(`Saved ${geometries.length} geofence(s)`, { type: "info" });
  };

  return (
    <div className="absolute bottom-4 left-1/2 z-10 flex -translate-x-1/2 gap-1 rounded-lg border bg-background/90 p-1 shadow-md backdrop-blur">
      {MODES.map((m) => (
        <Button
          key={m.mode}
          size="sm"
          variant={drawMode === m.mode ? "default" : "ghost"}
          onClick={() => setDrawMode(m.mode)}
        >
          {m.label}
        </Button>
      ))}
      <Button
        size="sm"
        variant="ghost"
        disabled={draftFeatures.features.length < 2}
        onClick={handleMerge}
      >
        Merge
      </Button>
      <Button
        size="sm"
        variant="ghost"
        disabled={draftFeatures.features.length === 0}
        onClick={handleSave}
      >
        Save
      </Button>
      <Button size="sm" variant="ghost" onClick={clearDraft}>Cancel</Button>
    </div>
  );
}
