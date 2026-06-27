import { DeckCanvas } from "@/map/deck-canvas";
import { LayerDrawer } from "@/map/panels/layer-drawer";
import { CoordinateReadout } from "@/map/panels/coordinate-readout";
import { TileServerSelect } from "@/map/panels/tile-server-select";
import { SelectionPopup } from "@/map/panels/selection-popup";

export function MapRoute() {
  return (
    <div className="relative h-screen w-screen overflow-hidden">
      <DeckCanvas />
      <TileServerSelect />
      <LayerDrawer />
      <CoordinateReadout />
      <SelectionPopup />
    </div>
  );
}
