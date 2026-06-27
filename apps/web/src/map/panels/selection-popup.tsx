import { Button } from "@/components/ui/button";
import { useMapUIStore } from "@/map/stores/map-ui-store";

/** Leaf: subscribes to selection only. Phase 1 shows kind+id; deep-link to the
 *  resource edit page lands with editing (Phase 2). */
export function SelectionPopup() {
  const selection = useMapUIStore((s) => s.selection);
  const setSelection = useMapUIStore((s) => s.setSelection);
  if (!selection.kind) return null;
  return (
    <div className="absolute bottom-4 right-4 z-10 w-56 rounded-lg border bg-background/95 p-3 shadow-md">
      <p className="text-sm font-medium capitalize">{selection.kind}</p>
      <p className="text-xs text-muted-foreground">#{selection.id}</p>
      <Button size="sm" variant="ghost" className="mt-2" onClick={() => setSelection({ kind: null, id: null })}>
        Close
      </Button>
    </div>
  );
}
