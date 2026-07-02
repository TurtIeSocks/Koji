import { Button } from "@/components/ui/button";
import { useMapUIStore } from "@/map/stores/map-ui-store";

/** Leaf: subscribes to selection + the clicked feature. Geofences go straight
 *  into the editor on click (no popup); this shows the name for markers/routes
 *  (a selected route is also the reroute/route-stats calc input). */
export function SelectionPopup() {
  const selection = useMapUIStore((s) => s.selection);
  const selectedFeature = useMapUIStore((s) => s.selectedFeature);
  const setSelection = useMapUIStore((s) => s.setSelection);
  if (!selection.kind) return null;

  const name =
    typeof selectedFeature?.properties?.name === "string" ? selectedFeature.properties.name : null;

  return (
    <div className="absolute bottom-4 right-4 z-10 w-56 rounded-lg border bg-background/95 p-3 shadow-md">
      <p className="text-sm font-medium capitalize">{selection.kind}</p>
      <p className="text-xs text-muted-foreground">{name ?? `#${selection.id}`}</p>
      <div className="mt-2 flex gap-2">
        <Button size="sm" variant="ghost" onClick={() => setSelection({ kind: null, id: null })}>
          Close
        </Button>
      </div>
    </div>
  );
}
