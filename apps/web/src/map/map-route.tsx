import { ArrowLeft } from "lucide-react";
import { Link } from "react-router";
import { Button } from "@/components/ui/button";
import { DeckCanvas } from "@/map/deck-canvas";
import { LayerDrawer } from "@/map/panels/layer-drawer";
import { CoordinateReadout } from "@/map/panels/coordinate-readout";
import { TileServerSelect } from "@/map/panels/tile-server-select";
import { SelectionPopup } from "@/map/panels/selection-popup";
import { DrawToolbar } from "@/map/panels/draw-toolbar";
import { FilterPanel } from "@/map/panels/filter-panel";

export function MapRoute() {
  return (
    <div className="relative h-screen w-screen overflow-hidden">
      <DeckCanvas />
      {/* Full-bleed (no admin sidebar) → top-left toolbar: back-to-admin + tile switch. */}
      <div className="absolute top-4 left-4 z-10 flex items-center gap-2">
        <Button asChild size="sm" variant="secondary" className="bg-background/90 shadow-md backdrop-blur">
          <Link to="/">
            <ArrowLeft className="size-4" /> Admin
          </Link>
        </Button>
        <TileServerSelect />
      </div>
      {/* Top-right column: layers card, then the filter card stacked below it. */}
      <div className="absolute top-4 right-4 z-10 flex w-56 flex-col gap-2">
        <LayerDrawer />
        <FilterPanel />
      </div>
      <CoordinateReadout />
      <SelectionPopup />
      <DrawToolbar />
    </div>
  );
}
