import { Button } from "@/components/ui/button";
import { useMapUIStore } from "@/map/stores/map-ui-store";
import { firstGeometry } from "@/map/lib/edit-serialize";
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
  const clearDraft = useMapUIStore((s) => s.clearDraft);
  const dataProvider = useDataProvider();
  const notify = useNotify();

  const handleSave = async () => {
    const geometry = firstGeometry(draftFeatures);
    if (!geometry) return;
    await dataProvider.create("geofence", {
      data: { name: `map-${Date.now()}`, mode: "Unset", geometry },
    });
    clearDraft();
    notify("Geofence saved", { type: "info" });
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
        disabled={draftFeatures.features.length === 0}
        onClick={handleSave}
      >
        Save
      </Button>
      <Button size="sm" variant="ghost" onClick={clearDraft}>Cancel</Button>
    </div>
  );
}
