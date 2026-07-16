import type { Layer } from "@deck.gl/core";
import { GeoJsonLayer } from "@deck.gl/layers";
import {
	Circle,
	Eraser,
	type LucideIcon,
	Move,
	Pentagon,
	Redo2,
	Scissors,
	Spline,
	Square,
	Trash2,
	Undo2,
} from "lucide-react";
import React, { type ReactNode, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useWatch } from "react-hook-form";
import { Button } from "@/components/ui/button";
import { createModeInstance, type DrawMode, requiresPriorSelection } from "@/map/lib/edit-modes";
import { buildEditLayer } from "@/map/lib/layers";
import { ButtonGroup, ButtonGroupSeparator } from "../ui/button-group";
import { Tooltip, TooltipContent, TooltipTrigger } from "../ui/tooltip";
import { valueBounds } from "./bounds";
import { DeckMap, type DeckMapProps } from "./deck-map";
import type { HistoryControls } from "./use-geometry-history";
import {
	type UseDeckEditRHFOptions,
	useDeckEditRHF,
} from "./use-deck-edit-rhf";

const DRAFT_FILL: [number, number, number, number] = [0, 150, 255, 60];
const DRAFT_LINE: [number, number, number, number] = [0, 150, 255, 220];

// The full editable-layers toolset (edit-modes.ts), grouped, as icon buttons so
// all of it fits one row. split/cutHole draw against an already-selected shape,
// so they're gated on a selection below.
const DRAW_GROUPS: {
	name: string;
	buttons: { mode: DrawMode; label: string; Icon: LucideIcon }[];
}[] = [
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
	history,
	onDelete,
	allowedModes,
	drawing,
}: {
	mode: DrawMode;
	setMode: (m: DrawMode) => void;
	hasSelection: boolean;
	/** Optional undo/redo controls docked at the toolbar's left. */
	history?: HistoryControls;
	/** Delete the selected shape(s). Gated on `hasSelection`. */
	onDelete: () => void;
	allowedModes?: DrawMode[];
	/** In-progress polygon draw (drawPolygon only) — Done/Cancel buttons. */
	drawing?: { canFinish: boolean; onDone: () => void; onCancel: () => void } | null;
}) {
	const groups = allowedModes
		? DRAW_GROUPS.map((g) => ({
				...g,
				buttons: g.buttons.filter((b) => allowedModes.includes(b.mode)),
			})).filter((g) => g.buttons.length > 0)
		: DRAW_GROUPS;

	return (
		// Flush bottom dock — sits on the map's bottom edge (no gap to the sides or
		// bottom), a top border rather than a shadow so it reads as a bar BELOW the
		// map, not a card floating on top.
		<div className="absolute inset-x-0 bottom-0 z-10 flex items-center gap-2 border-t bg-background/95 px-2 py-1.5 backdrop-blur">
			{history ? (
				<ButtonGroup>
					<Tooltip>
						<TooltipTrigger asChild>
							<Button
								type="button"
								size="icon-sm"
								variant="ghost"
								disabled={!history.canUndo}
								aria-label="Undo"
								onClick={history.undo}
							>
								<Undo2 />
							</Button>
						</TooltipTrigger>
						<TooltipContent>Undo</TooltipContent>
					</Tooltip>
					<Tooltip>
						<TooltipTrigger asChild>
							<Button
								type="button"
								size="icon-sm"
								variant="ghost"
								disabled={!history.canRedo}
								aria-label="Redo"
								onClick={history.redo}
							>
								<Redo2 />
							</Button>
						</TooltipTrigger>
						<TooltipContent>Redo</TooltipContent>
					</Tooltip>
				</ButtonGroup>
			) : null}
			<ButtonGroup className="flex-1 justify-evenly">
				{groups.map((group, gi) => (
					<React.Fragment key={group.name}>
						{gi > 0 ? <ButtonGroupSeparator /> : null}
						{group.buttons.map((b) => {
							// split / cutHole draw ONTO a selected shape → disabled until one is
							// selected (modify/transform select on click; drawing selects nothing).
							const disabled = requiresPriorSelection(b.mode) && !hasSelection;
							return (
								<Tooltip key={b.mode}>
									<TooltipTrigger asChild>
										<Button
											key={b.mode}
											type="button"
											size="icon-sm"
											variant={mode === b.mode ? "default" : "ghost"}
											disabled={disabled}
											aria-label={b.label}
											onClick={() => setMode(mode === b.mode ? "none" : b.mode)}
										>
											<b.Icon />
										</Button>
									</TooltipTrigger>
									<TooltipContent>
										{disabled ? "Select a shape first" : b.label}
									</TooltipContent>
								</Tooltip>
							);
						})}
					</React.Fragment>
				))}
			</ButtonGroup>
			<Tooltip>
				<TooltipTrigger asChild>
					<Button
						type="button"
						size="icon-sm"
						variant="ghost"
						disabled={!hasSelection}
						aria-label="Delete shape"
						className="text-destructive hover:text-destructive"
						onClick={onDelete}
					>
						<Trash2 />
					</Button>
				</TooltipTrigger>
				<TooltipContent>
					{hasSelection ? "Delete selected shape" : "Select a shape first"}
				</TooltipContent>
			</Tooltip>
			{drawing ? (
				<ButtonGroup>
					<Button
						type="button"
						size="sm"
						variant="default"
						disabled={!drawing.canFinish}
						aria-label="Finish drawing"
						onClick={drawing.onDone}
					>
						Done
					</Button>
					<Button
						type="button"
						size="sm"
						variant="ghost"
						aria-label="Cancel drawing"
						onClick={drawing.onCancel}
					>
						Cancel
					</Button>
				</ButtonGroup>
			) : null}
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
	/** Initial camera for the EMPTY case (no geometry yet, e.g. a create form) so
	 *  it doesn't open at [0,0]. Ignored once there's geometry to fit. */
	defaultViewState?: { longitude: number; latitude: number; zoom?: number };
	/** Extra absolutely-positioned overlay children on the map (e.g. a calc panel
	 *  dock) rendered alongside the draw toolbar. */
	overlay?: ReactNode;
	/** Undo/redo controls for the draw toolbar (playground only). */
	history?: HistoryControls;
	/** Observe camera pan/zoom (e.g. to persist it). Forwarded to `<DeckMap>`. */
	onViewStateChange?: DeckMapProps["onViewStateChange"];
	/** Forwarded straight through to `<DeckMap>` (e.g. a "Show Neighbors"
	 *  ghost-fence overlay's tooltip). Optional/back-compat. */
	getTooltip?: DeckMapProps["getTooltip"];
	/** Restrict which draw/edit modes the toolbar offers (e.g. the import wizard
	 *  hides transform/split/cutHole — they'd desync per-feature assignments).
	 *  Omitted = all modes. */
	allowedModes?: DrawMode[];
}

export function DeckGeoJsonInput({
	label,
	helperText,
	height = 400,
	tileUrl,
	disabled,
	contextLayers,
	defaultViewState,
	overlay,
	history,
	onViewStateChange,
	getTooltip,
	allowedModes,
	...editOpts
}: DeckGeoJsonInputProps) {
	const {
		draft,
		mode,
		setMode,
		selectedIndexes,
		onEdit,
		onSelect,
		deleteSelected,
		version,
	} = useDeckEditRHF(editOpts);

	// Live vertex count for the in-progress polygon (Done enables at 3). The
	// mode's internal clickSequence isn't reactive — count its
	// addTentativePosition edits instead; any real edit or cancel resets.
	const [tentativeCount, setTentativeCount] = useState(0);
	const tentativeCountRef = useRef(0);
	tentativeCountRef.current = tentativeCount;
	const onEditWrapped = useCallback(
		(e: Parameters<typeof onEdit>[0]) => {
			if (e.editType === "addTentativePosition") setTentativeCount((c) => c + 1);
			else if (e.editType !== "updateTentativeFeature") setTentativeCount(0);
			onEdit(e);
		},
		[onEdit],
	);
	const setModeReset = useCallback(
		(m: DrawMode) => {
			setTentativeCount(0);
			setMode(m);
		},
		[setMode],
	);

	// A fresh mode INSTANCE per mode switch (not the class) — editable-geojson-layer
	// only re-instantiates its internal mode when this prop's identity changes, so
	// the memoized instance (and its in-progress click sequence) survives editLayers
	// rebuilds triggered by every other draft change (selection, onEdit, version…).
	const modeInstance = useMemo(() => createModeInstance(mode), [mode]);
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
			return buildEditLayer({
				mode,
				features: draft,
				selectedIndexes,
				onEdit: onEditWrapped,
				onSelect,
				version,
				modeInstance,
			});
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
	}, [mode, draft, selectedIndexes, onEditWrapped, onSelect, disabled, version, modeInstance]);
	// Neighbors (contextLayers) are only clickable when the draw tool is idle —
	// while drawing is actually active, force them non-pickable so a click meant
	// to draw (or a hover mid-drag) can't collide with the neighbor overlay's own
	// onClick/tooltip picking. "Drawing active" mirrors the editLayer gate above
	// (`mode !== "none" && !disabled`): a hydrated-geometry auto-`modify` mode on a
	// `disabled` (read-only) input isn't drawing, so neighbors stay clickable.
	const layers = useMemo(() => {
		const drawing = mode !== "none" && !disabled;
		const ctx = drawing
			? (contextLayers ?? []).map((l) => l.clone({ pickable: false }))
			: (contextLayers ?? []);
		return [...ctx, ...editLayers];
	}, [contextLayers, editLayers, mode, disabled]);
	// Frame the map on the whole stored geometry. Read it from the FORM VALUE
	// (available synchronously at mount) rather than `draft` — the draft hydrates
	// in a post-mount effect, so it's empty when DeckMap locks its initial camera,
	// which left the map stuck at [0,0].
	const value = useWatch({ name: editOpts.source });
	const fit = useMemo(() => valueBounds(value), [value]);

	// Escape cancels an in-progress polygon draw. Registered on the CAPTURE
	// phase + stopPropagation so it wins over DeckMap's own window-keydown
	// Escape listener (fullscreen-collapse, deck-map.tsx:76-83) while vertices
	// are down; with none down, the event passes through untouched so
	// fullscreen-collapse still works.
	useEffect(() => {
		if (mode !== "drawPolygon" || disabled) return;
		const h = (e: KeyboardEvent) => {
			if (e.key !== "Escape" || tentativeCountRef.current === 0) return;
			e.stopPropagation(); // beat DeckMap's fullscreen-collapse listener
			(modeInstance as { cancel?: () => void }).cancel?.();
			setTentativeCount(0);
		};
		window.addEventListener("keydown", h, true);
		return () => window.removeEventListener("keydown", h, true);
	}, [mode, disabled, modeInstance]);

	return (
		<div className="flex flex-col gap-1" data-slot="deck-geojson-input">
			{label ? <span className="text-sm font-medium">{label}</span> : null}
			<div className="relative" style={{ height }}>
				<DeckMap
					layers={layers}
					fitBounds={fit}
					initialViewState={fit ? undefined : defaultViewState}
					height={height}
					tileUrl={tileUrl}
					controller={{ doubleClickZoom: false }}
					onViewStateChange={onViewStateChange}
					getTooltip={getTooltip}
					getCursor={({ isDragging }) =>
						mode !== "none" ? "crosshair" : isDragging ? "grabbing" : "grab"
					}
				>
					{!disabled ? (
						<DeckDrawToolbar
							mode={mode}
							setMode={setModeReset}
							hasSelection={selectedIndexes.length > 0}
							history={history}
							onDelete={deleteSelected}
							allowedModes={allowedModes}
							drawing={
								mode === "drawPolygon"
									? {
											canFinish: tentativeCount >= 3,
											onDone: () => {
												(modeInstance as { finish?: () => void }).finish?.();
												setTentativeCount(0);
											},
											onCancel: () => {
												(modeInstance as { cancel?: () => void }).cancel?.();
												setTentativeCount(0);
											},
										}
									: null
							}
						/>
					) : null}
					{(mode === "modify" || mode === "transform") &&
					selectedIndexes.length === 0 &&
					draft.features.length > 0 ? (
						<div className="pointer-events-none absolute inset-x-0 top-2 z-10 flex justify-center">
							<span className="rounded-md bg-background/95 px-3 py-1 text-sm text-muted-foreground shadow">
								Click a shape to edit its points
							</span>
						</div>
					) : null}
					{overlay}
				</DeckMap>
			</div>
			{helperText ? (
				<div className="text-xs text-muted-foreground">{helperText}</div>
			) : null}
		</div>
	);
}
