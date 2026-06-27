import { Button } from "@/components/ui/button";
import { useMapUIStore } from "@/map/stores/map-ui-store";
import type { DrawMode } from "@/map/lib/edit-modes";

const MODES: { mode: DrawMode; label: string }[] = [
  { mode: "drawPolygon", label: "Polygon" },
  { mode: "drawRectangle", label: "Rectangle" },
  { mode: "modify", label: "Modify" },
  { mode: "translate", label: "Move" },
];

/** Leaf: subscribes to drawMode + the edit actions only (S3). Save is wired by Task 4. */
export function DrawToolbar() {
  const drawMode = useMapUIStore((s) => s.drawMode);
  const setDrawMode = useMapUIStore((s) => s.setDrawMode);
  const clearDraft = useMapUIStore((s) => s.clearDraft);
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
      <Button size="sm" variant="ghost" onClick={clearDraft}>Cancel</Button>
    </div>
  );
}
