import { type ReactNode, useMemo } from "react";
import { useWatch } from "react-hook-form";
import type { Layer } from "@deck.gl/core";
import { GeoJsonLayer } from "@deck.gl/layers";
import { buildEditLayer } from "@/map/lib/layers";
import { requiresPriorSelection, type DrawMode } from "@/map/lib/edit-modes";
import { Button } from "@/components/ui/button";
import { Pentagon, Square, Circle, Spline, Move, Scissors, Eraser, type LucideIcon } from "lucide-react";
import { DeckMap } from "./deck-map";
import { geometryBounds } from "./bounds";
import { useDeckEditRHF, type UseDeckEditRHFOptions } from "./use-deck-edit-rhf";

const DRAFT_FILL: [number, number, number, number] = [0, 150, 255, 60];
const DRAFT_LINE: [number, number, number, number] = [0, 150, 255, 220];

// The full editable-layers toolset (edit-modes.ts), grouped, as icon buttons so
// all of it fits one row. split/cutHole draw against an already-selected shape,
// so they're gated on a selection below.
const DRAW_GROUPS: { name: string; buttons: { mode: DrawMode; label: string; Icon: LucideIcon }[] }[] = [
  {
    name: "draw",
    buttons: [
      { mode: "drawPolygon", label: "Polygon", Icon: Pentagon },
      { mode: "drawRectangle", label: "Rectangle", Icon: Square },
      { mode: "drawCircle", label: "Circle", Icon: Circle },
    ],
  },
  {
    name: "edit",
    buttons: [
      { mode: "modify", label: "Modify", Icon: Spline },
      { mode: "transform", label: "Move", Icon: Move },
    ],
  },
  {
    name: "shape-ops",
    buttons: [
      { mode: "split", label: "Split", Icon: Scissors },
      { mode: "cutHole", label: "Cut hole", Icon: Eraser },
    ],
  },
];

function DeckDrawToolbar({
  mode,
  setMode,
  hasSelection,
}: {
  mode: DrawMode;
  setMode: (m: DrawMode) => void;
  hasSelection: boolean;
}) {
  return (
    // Flush bottom dock — sits on the map's bottom edge (no gap to the sides or
    // bottom), a top border rather than a shadow so it reads as a bar BELOW the
    // map, not a card floating on top.
    <div className="absolute inset-x-0 bottom-0 z-10 flex items-center gap-1 border-t bg-background/95 px-2 py-1.5 backdrop-blur">
      {DRAW_GROUPS.map((group, gi) => (
        <div key={group.name} className="flex items-center gap-1">
          {gi > 0 ? <div className="mx-1 h-5 w-px shrink-0 bg-border" /> : null}
          {group.buttons.map((b) => {
            // split / cutHole draw ONTO a selected shape → disabled until one is
            // selected (modify/transform select on click; drawing selects nothing).
            const disabled = requiresPriorSelection(b.mode) && !hasSelection;
            return (
              <Button
                key={b.mode}
                type="button"
                size="icon-sm"
                variant={mode === b.mode ? "default" : "ghost"}
                disabled={disabled}
                aria-label={b.label}
                title={disabled ? "Select a shape first" : b.label}
                onClick={() => setMode(mode === b.mode ? "none" : b.mode)}
              >
                <b.Icon />
              </Button>
            );
          })}
        </div>
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
  // buildEditLayer short-circuits to [] whenever mode === "none" (the default,
  // and the only state while `disabled`) — without a fallback here, a
  // hydrated draft (e.g. editing an existing geofence) would render as a
  // blank map. Draw a static read-only GeoJsonLayer for the draft instead,
  // same treatment as <DeckGeoJsonField>.
  const editLayers = useMemo<Layer[]>(() => {
    // `disabled` forces read-only, overriding the auto-modify default so a
    // disabled input never becomes editable.
    if (mode !== "none" && !disabled) {
      return buildEditLayer({ mode, features: draft, selectedIndexes, onEdit, onSelect });
    }
    if (draft.features.length === 0) return [];
    return [
      new GeoJsonLayer({
        id: "edit-static",
        data: draft,
        filled: true,
        getFillColor: DRAFT_FILL,
        stroked: true,
        getLineColor: DRAFT_LINE,
        lineWidthMinPixels: 2,
        pointType: "circle",
        getPointRadius: 5,
        pointRadiusUnits: "pixels",
      }),
    ];
  }, [mode, draft, selectedIndexes, onEdit, onSelect, disabled]);
  const layers = useMemo(() => [...(contextLayers ?? []), ...editLayers], [contextLayers, editLayers]);
  // Frame the map on the whole stored geometry. Read it from the FORM VALUE
  // (available synchronously at mount) rather than `draft` — the draft hydrates
  // in a post-mount effect, so it's empty when DeckMap locks its initial camera,
  // which left the map stuck at [0,0].
  const value = useWatch({ name: editOpts.source });
  const fit = useMemo(() => {
    const g = value as GeoJSON.GeoJSON | null | undefined;
    return g ? geometryBounds(g) : null;
  }, [value]);

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
          {!disabled ? (
            <DeckDrawToolbar mode={mode} setMode={setMode} hasSelection={selectedIndexes.length > 0} />
          ) : null}
        </DeckMap>
      </div>
      {helperText ? <div className="text-xs text-muted-foreground">{helperText}</div> : null}
    </div>
  );
}
