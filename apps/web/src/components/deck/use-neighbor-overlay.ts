import type { Layer, PickingInfo } from "@deck.gl/core";
import { useMemo, useState } from "react";
import type { NeighborFilters } from "@/map/data/use-geo-features";
import { useGeofencesByBbox } from "@/map/data/use-geo-features";
import { geofenceOverlayLayer, overlayTooltip } from "@/map/lib/geofence-overlay";
import type { Bounds } from "@/map/stores/types";

export interface UseNeighborOverlayResult {
	on: boolean;
	setOn: (v: boolean) => void;
	layers: Layer[];
	getTooltip: (info: PickingInfo) => { text: string } | null;
	label: string;
}

/** Expand a bbox by `frac * span` on each axis, on both sides. Null-safe —
 *  passes through `null`/`undefined` bounds as `null`. Exported for direct
 *  unit testing. */
export function padBbox(b: Bounds | null | undefined, frac: number): Bounds | null {
	if (!b) return null;
	const [minX, minY, maxX, maxY] = b;
	const dx = (maxX - minX) * frac;
	const dy = (maxY - minY) * frac;
	return [minX - dx, minY - dy, maxX + dx, maxY + dy];
}

/** "Show Neighbors" toggle — reveals dimmed ghost fences inside `bbox` for
 *  overlap visualization. The CALLER derives the bbox: camera viewport on
 *  create/edit (so it works before anything is drawn — beta feedback
 *  2026-07-15), padded record-geometry on the show page. Default OFF.
 *
 *  Calls `useGeofencesByBbox` UNCONDITIONALLY every render (fixed hook order
 *  per the rules of hooks) — the fetch itself is gated via its own `enabled`
 *  arg (`on && !!bbox`) instead of being skipped. */
export function useNeighborOverlay(
	bbox: Bounds | null,
	currentId?: number | string | null,
	filters?: NeighborFilters,
): UseNeighborOverlayResult {
	const [on, setOn] = useState(false);

	const q = useGeofencesByBbox(bbox, on && !!bbox, filters);

	const layers = useMemo<Layer[]>(
		() =>
			on && q.data
				? [geofenceOverlayLayer({ features: q.data.features, ghost: true, excludeId: currentId })]
				: [],
		[on, q.data, currentId],
	);

	return { on, setOn, layers, getTooltip: overlayTooltip, label: "Neighbors" };
}
