import { Switch } from "@/components/ui/switch";
import { Label } from "@/components/ui/label";
import { useMapUIStore } from "@/map/stores/map-ui-store";
import type { LayerId } from "@/map/stores/types";

const LAYERS: { id: LayerId; label: string }[] = [
  { id: "geofences", label: "Geofences" }, { id: "routes", label: "Routes" },
  { id: "gyms", label: "Gyms" }, { id: "pokestops", label: "Pokestops" },
  { id: "spawnpoints", label: "Spawnpoints" }, { id: "stations", label: "Stations" },
  { id: "s2", label: "S2 cells" },
];

/** Leaf: subscribes ONLY to its own boolean (S3 — render isolation per toggle). */
function LayerToggle({ id, label }: { id: LayerId; label: string }) {
  const checked = useMapUIStore((s) => s.layerVisibility[id]);
  const toggleLayer = useMapUIStore((s) => s.toggleLayer);
  return (
    <div className="flex items-center justify-between gap-3 py-1">
      <Label htmlFor={`layer-${id}`}>{label}</Label>
      <Switch id={`layer-${id}`} aria-label={label} checked={checked} onCheckedChange={() => toggleLayer(id)} />
    </div>
  );
}

export function LayerDrawer() {
  return (
    <div className="absolute top-4 right-4 z-10 w-56 rounded-lg border bg-background/90 p-3 shadow-md backdrop-blur">
      <p className="mb-2 text-sm font-medium">Layers</p>
      {LAYERS.map((l) => <LayerToggle key={l.id} id={l.id} label={l.label} />)}
    </div>
  );
}
