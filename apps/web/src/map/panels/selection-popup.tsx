import { Button } from "@/components/ui/button";
import { useMapUIStore } from "@/map/stores/map-ui-store";

/** Leaf: subscribes to selection + the clicked feature. For a geofence it offers
 *  "Edit geometry", which loads it into the modify editor (see editFeature). */
export function SelectionPopup() {
  const selection = useMapUIStore((s) => s.selection);
  const selectedFeature = useMapUIStore((s) => s.selectedFeature);
  const setSelection = useMapUIStore((s) => s.setSelection);
  const editFeature = useMapUIStore((s) => s.editFeature);
  if (!selection.kind) return null;

  const name =
    typeof selectedFeature?.properties?.name === "string" ? selectedFeature.properties.name : null;

  return (
    <div className="absolute bottom-4 right-4 z-10 w-56 rounded-lg border bg-background/95 p-3 shadow-md">
      <p className="text-sm font-medium capitalize">{selection.kind}</p>
      <p className="text-xs text-muted-foreground">{name ?? `#${selection.id}`}</p>
      <div className="mt-2 flex gap-2">
        {selection.kind === "geofence" && selectedFeature && (
          <Button size="sm" variant="default" onClick={() => editFeature(selectedFeature)}>
            Edit geometry
          </Button>
        )}
        <Button size="sm" variant="ghost" onClick={() => setSelection({ kind: null, id: null })}>
          Close
        </Button>
      </div>
    </div>
  );
}
