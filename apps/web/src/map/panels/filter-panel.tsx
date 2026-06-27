import { Label } from "@/components/ui/label";
import { Slider } from "@/components/ui/slider";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { useMapUIStore } from "@/map/stores/map-ui-store";

/** Last-seen slider: subscribes to filters.lastSeen + setLastSeen (S3). */
function LastSeenSlider() {
  const lastSeen = useMapUIStore((s) => s.filters.lastSeen);
  const setLastSeen = useMapUIStore((s) => s.setLastSeen);
  const hours = lastSeen === 0 ? "All" : `${Math.round(lastSeen / 3600)}h`;
  return (
    <div className="flex flex-col gap-1">
      <Label className="text-xs text-muted-foreground">Last seen: {hours}</Label>
      <Slider
        min={0}
        max={24 * 3600}
        step={3600}
        value={[lastSeen]}
        onValueChange={([v]) => setLastSeen(v)}
        className="w-36"
        aria-label="Last seen"
      />
    </div>
  );
}

/** TTH select: subscribes to filters.tth + setTth (S3). */
function TthSelect() {
  const tth = useMapUIStore((s) => s.filters.tth);
  const setTth = useMapUIStore((s) => s.setTth);
  return (
    <div className="flex flex-col gap-1">
      <Label className="text-xs text-muted-foreground">Spawnpoint TTH</Label>
      <Select value={tth} onValueChange={(v) => setTth(v as "All" | "Known" | "Unknown")}>
        <SelectTrigger className="w-36">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          <SelectItem value="All">All</SelectItem>
          <SelectItem value="Known">Known</SelectItem>
          <SelectItem value="Unknown">Unknown</SelectItem>
        </SelectContent>
      </Select>
    </div>
  );
}

/** FilterPanel: server re-query filters for last-seen + spawnpoint TTH. */
export function FilterPanel() {
  return (
    <div className="flex w-full flex-col gap-3 rounded-lg border bg-background/90 p-3 shadow-md backdrop-blur">
      <LastSeenSlider />
      <TthSelect />
    </div>
  );
}
