import type { Layer, PickingInfo } from "@deck.gl/core";
import { useMemo, useState } from "react";
import { useGeofencesByBbox } from "@/map/data/use-geo-features";
import { geofenceOverlayLayer, overlayTooltip } from "@/map/lib/geofence-overlay";
import type { Bounds } from "@/map/stores/types";
import { geometryBounds } from "./bounds";

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

/** "Show Neighbors" toggle for a geofence show/edit/create page — reveals
 *  dimmed ghost fences near `geometry` (a padded-bbox scoped fetch) for
 *  overlap visualization. Mirrors `useMarkerOverlay`'s toggle+layer pattern.
 *  Default OFF.
 *
 *  Calls `useGeofencesByBbox` UNCONDITIONALLY every render (fixed hook order
 *  per the rules of hooks) — the fetch itself is gated via its own `enabled`
 *  arg (`on && !!geometry`) instead of being skipped. */
export function useNeighborOverlay(
	geometry: GeoJSON.Geometry | null | undefined,
	currentId?: number | string | null,
): UseNeighborOverlayResult {
	const [on, setOn] = useState(false);

	const bbox = geometry ? padBbox(geometryBounds(geometry), 0.2) : null;
	const q = useGeofencesByBbox(bbox, on && !!geometry);

	const layers = useMemo<Layer[]>(
		() =>
			on && q.data
				? [geofenceOverlayLayer({ features: q.data.features, ghost: true, excludeId: currentId })]
				: [],
		[on, q.data, currentId],
	);

	return { on, setOn, layers, getTooltip: overlayTooltip, label: "Neighbors" };
}
