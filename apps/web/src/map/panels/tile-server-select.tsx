import {
  Select, SelectContent, SelectItem, SelectTrigger, SelectValue,
} from "@/components/ui/select";
import { useMapSettingsStore } from "@/map/stores/map-settings-store";

/** Phase 1: single "default" option (DEFAULT_TILE_URL). Real tile-server list
 *  from the tile_server resource lands when the switch needs >1 source. */
export function TileServerSelect() {
  const tileServerId = useMapSettingsStore((s) => s.tileServerId);
  const setTileServerId = useMapSettingsStore((s) => s.setTileServerId);
  // Positioned by the parent (MapRoute) so it can sit in the top-left toolbar row.
  return (
    <Select value={tileServerId} onValueChange={setTileServerId}>
      <SelectTrigger className="w-40 bg-background/90"><SelectValue /></SelectTrigger>
      <SelectContent><SelectItem value="default">Default</SelectItem></SelectContent>
    </Select>
  );
}
