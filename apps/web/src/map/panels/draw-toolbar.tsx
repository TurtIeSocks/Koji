import type { ComponentType } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { Button } from "@/components/ui/button";
import { Pentagon, Square, Circle, Spline, Move, Donut, Scissors, Combine, Check, X } from "lucide-react";
import { useMapUIStore } from "@/map/stores/map-ui-store";
import { allGeometries } from "@/map/lib/edit-serialize";
import { mergeSelected } from "@/map/lib/merge-polygons";
import { requiresPriorSelection, type DrawMode } from "@/map/lib/edit-modes";
import { useDataProvider, useNotify } from "shadmin-core";

interface ModeButtonDef {
  mode: DrawMode;
  label: string;
  Icon: ComponentType<{ className?: string }>;
}

const DRAW_MODES: ModeButtonDef[] = [
  { mode: "drawPolygon", label: "Polygon", Icon: Pentagon },
  { mode: "drawRectangle", label: "Rectangle", Icon: Square },
  { mode: "drawCircle", label: "Circle", Icon: Circle },
];
const EDIT_MODES: ModeButtonDef[] = [
  { mode: "modify", label: "Modify vertices", Icon: Spline },
  { mode: "transform", label: "Move / rotate / scale", Icon: Move },
  { mode: "cutHole", label: "Cut hole", Icon: Donut },
  { mode: "split", label: "Split", Icon: Scissors },
];

/** Leaf: subscribes to drawMode / draft / selection + the edit actions only (S3). */
export function DrawToolbar() {
  const drawMode = useMapUIStore((s) => s.drawMode);
  const draftFeatures = useMapUIStore((s) => s.draftFeatures);
  const selectedFeatureIndexes = useMapUIStore((s) => s.selectedFeatureIndexes);
  const setDrawMode = useMapUIStore((s) => s.setDrawMode);
  const setDraftFeatures = useMapUIStore((s) => s.setDraftFeatures);
  const setSelectedFeatureIndexes = useMapUIStore((s) => s.setSelectedFeatureIndexes);
  const editingGeofenceId = useMapUIStore((s) => s.editingGeofenceId);
  const clearDraft = useMapUIStore((s) => s.clearDraft);
  const dataProvider = useDataProvider();
  const notify = useNotify();
  const queryClient = useQueryClient();

  // Refetch the geofence layer right after our OWN save committed, rather than
  // waiting on the realtime `resource/geofence` event (which races the DB commit
  // and intermittently drops the just-saved shape until a manual refresh).
  const refetchGeofences = () => void queryClient.invalidateQueries({ queryKey: ["geo", "geofences"] });

  const hasSelection = selectedFeatureIndexes.length > 0;

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
      refetchGeofences();
      clearDraft();
      notify(rest.length > 0 ? `Geofence updated (+${rest.length} new)` : "Geofence updated", { type: "info" });
      return;
    }
    // New shapes → one geofence per drawn feature.
    await createGeometries(geometries);
    refetchGeofences();
    clearDraft();
    notify(`Saved ${geometries.length} geofence(s)`, { type: "info" });
  };

  const renderMode = (m: ModeButtonDef) => {
    // Cut-hole / split need a shape ALREADY selected (their clicks draw, not
    // select) — gate them until the user has picked a target via modify/transform.
    const gated = requiresPriorSelection(m.mode) && !hasSelection;
    return (
      <Button
        key={m.mode}
        size="icon"
        variant={drawMode === m.mode ? "default" : "ghost"}
        aria-label={m.label}
        title={gated ? `${m.label} — select a shape first` : m.label}
        disabled={gated}
        onClick={() => setDrawMode(m.mode)}
      >
        <m.Icon className="size-4" />
      </Button>
    );
  };

  return (
    <div className="absolute bottom-4 left-1/2 z-10 flex -translate-x-1/2 items-center gap-1 rounded-lg border bg-background/90 p-1 shadow-md backdrop-blur">
      {DRAW_MODES.map(renderMode)}
      <div className="mx-0.5 w-px self-stretch bg-border" />
      {EDIT_MODES.map(renderMode)}
      <div className="mx-0.5 w-px self-stretch bg-border" />
      <Button
        size="icon"
        variant="ghost"
        aria-label="Merge"
        title="Merge all shapes"
        disabled={draftFeatures.features.length < 2}
        onClick={handleMerge}
      >
        <Combine className="size-4" />
      </Button>
      <Button
        size="icon"
        variant="ghost"
        aria-label="Save"
        title="Save"
        disabled={draftFeatures.features.length === 0}
        onClick={handleSave}
      >
        <Check className="size-4" />
      </Button>
      <Button size="icon" variant="ghost" aria-label="Cancel" title="Cancel" onClick={clearDraft}>
        <X className="size-4" />
      </Button>
    </div>
  );
}
