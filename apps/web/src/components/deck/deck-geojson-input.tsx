import { type ReactNode, useMemo } from "react";
import type { Layer } from "@deck.gl/core";
import { buildEditLayer } from "@/map/lib/layers";
import type { DrawMode } from "@/map/lib/edit-modes";
import { Button } from "@/components/ui/button";
import { DeckMap } from "./deck-map";
import { geometryBounds } from "./bounds";
import { useDeckEditRHF, type UseDeckEditRHFOptions } from "./use-deck-edit-rhf";

// DrawMode union (from edit-modes.ts) is:
//   "none" | "drawPolygon" | "drawRectangle" | "drawCircle" | "modify"
//   | "transform" | "split" | "cutHole"
// Core geofence-editing set only (split/cutHole need a prior selection — omit for v1).
const DRAW_BUTTONS: { mode: DrawMode; label: string }[] = [
  { mode: "drawPolygon", label: "Polygon" },
  { mode: "drawRectangle", label: "Rectangle" },
  { mode: "modify", label: "Modify" },
  { mode: "transform", label: "Move" },
];

function DeckDrawToolbar({
  mode,
  setMode,
}: {
  mode: DrawMode;
  setMode: (m: DrawMode) => void;
  selectedCount?: number;
}) {
  return (
    <div className="absolute top-2 left-2 z-10 flex gap-1 rounded-md bg-background/90 p-1 shadow-md backdrop-blur">
      {DRAW_BUTTONS.map((b) => (
        <Button
          key={b.mode}
          size="sm"
          variant={mode === b.mode ? "default" : "secondary"}
          onClick={() => setMode(mode === b.mode ? "none" : b.mode)}
        >
          {b.label}
        </Button>
      ))}
    </div>
  );
}

export interface DeckGeoJsonInputProps extends UseDeckEditRHFOptions {
  label?: ReactNode;
  helperText?: ReactNode;
  height?: number | string;
  tileUrl?: string;
  disabled?: boolean;
  /** Extra read-only layers drawn under the edit layer (e.g. marker/S2 context). */
  contextLayers?: Layer[];
}

export function DeckGeoJsonInput({
  label,
  helperText,
  height = 400,
  tileUrl,
  disabled,
  contextLayers,
  ...editOpts
}: DeckGeoJsonInputProps) {
  const { draft, mode, setMode, selectedIndexes, onEdit, onSelect } = useDeckEditRHF(editOpts);

  // buildEditLayer takes a single DraftInput (not separate args — the plan's
  // snippet used a spread signature that doesn't match apps/web/src/map/lib/layers.ts).
  const editLayers = useMemo<Layer[]>(
    () => buildEditLayer({ mode, features: draft, selectedIndexes, onEdit, onSelect }),
    [mode, draft, selectedIndexes, onEdit, onSelect],
  );
  const layers = useMemo(() => [...(contextLayers ?? []), ...editLayers], [contextLayers, editLayers]);
  const fit = draft.features[0]?.geometry ? geometryBounds(draft.features[0].geometry) : null;

  return (
    <div className="flex flex-col gap-1" data-slot="deck-geojson-input">
      {label ? <span className="text-sm font-medium">{label}</span> : null}
      <div className="relative" style={{ height }}>
        <DeckMap
          layers={layers}
          fitBounds={fit}
          height={height}
          tileUrl={tileUrl}
          controller={{ doubleClickZoom: false }}
          getCursor={({ isDragging }) => (mode !== "none" ? "crosshair" : isDragging ? "grabbing" : "grab")}
        >
          {!disabled ? <DeckDrawToolbar mode={mode} setMode={setMode} selectedCount={selectedIndexes.length} /> : null}
        </DeckMap>
      </div>
      {helperText ? <div className="text-xs text-muted-foreground">{helperText}</div> : null}
    </div>
  );
}
