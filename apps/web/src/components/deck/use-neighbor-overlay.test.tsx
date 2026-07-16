import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

// Silence React's "not configured to support act(...)" warning — normally set
// by `@testing-library/react`'s own environment setup, which this project
// doesn't depend on (see the hand-rolled `renderHook` below).
(
	globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT?: boolean }
).IS_REACT_ACT_ENVIRONMENT = true;

// Hoisted so both the vi.mock factory and the tests can drive it.
const { useGeofencesByBboxMock } = vi.hoisted(() => ({
	useGeofencesByBboxMock: vi.fn(),
}));

vi.mock("@/map/data/use-geo-features", () => ({
	useGeofencesByBbox: useGeofencesByBboxMock,
}));

import type { Bounds } from "@/map/stores/types";
import { padBbox, useNeighborOverlay } from "./use-neighbor-overlay";

// `@testing-library/react` isn't a dependency of this project — hand-roll the
// minimal `renderHook` shape (result ref + act) backed by `react-dom/client`
// + React 19's own `act`, matching `use-marker-overlay.test.tsx`.
function renderHook<T>(callback: () => T) {
	const result: { current: T } = { current: undefined as unknown as T };
	function Probe() {
		result.current = callback();
		return null;
	}
	const container = document.createElement("div");
	document.body.appendChild(container);
	let root: Root;
	act(() => {
		root = createRoot(container);
		root.render(<Probe />);
	});
	return {
		result,
		unmount: () => act(() => root.unmount()),
	};
}

const poly: GeoJSON.Polygon = {
	type: "Polygon",
	coordinates: [
		[
			[0, 0],
			[1, 0],
			[1, 1],
			[0, 1],
			[0, 0],
		],
	],
};

// Previously `padBbox(geometryBounds(poly), 0.2)` — now passed directly by
// the caller, since the hook no longer derives the bbox itself.
const bbox: Bounds = [-0.2, -0.2, 1.2, 1.2];

const CURRENT_ID = 42;

const FC: GeoJSON.FeatureCollection = {
	type: "FeatureCollection",
	features: [
		{
			type: "Feature",
			id: CURRENT_ID,
			properties: { id: CURRENT_ID, name: "self" },
			geometry: poly,
		},
		{
			type: "Feature",
			id: 99,
			properties: { id: 99, name: "neighbor" },
			geometry: poly,
		},
	],
};

let mounted: { unmount: () => void } | null = null;
afterEach(() => {
	mounted?.unmount();
	mounted = null;
});

describe("useNeighborOverlay", () => {
	beforeEach(() => {
		useGeofencesByBboxMock.mockReset();
		// Mirrors the real useGeofencesByBbox contract: `data` is undefined
		// until the (mocked) query is enabled and "resolves".
		useGeofencesByBboxMock.mockImplementation(
			(_bbox: unknown, enabled: boolean) => ({
				data: enabled ? FC : undefined,
			}),
		);
	});

	it("defaults off: no layers, label is Neighbors", () => {
		const rendered = renderHook(() => useNeighborOverlay(bbox, CURRENT_ID));
		mounted = rendered;

		expect(rendered.result.current.on).toBe(false);
		expect(rendered.result.current.layers).toHaveLength(0);
		expect(rendered.result.current.label).toBe("Neighbors");
	});

	it("setOn(true) with a non-null bbox builds one layer excluding currentId", () => {
		const rendered = renderHook(() => useNeighborOverlay(bbox, CURRENT_ID));
		mounted = rendered;

		act(() => {
			rendered.result.current.setOn(true);
		});

		expect(rendered.result.current.on).toBe(true);
		expect(rendered.result.current.layers).toHaveLength(1);
		const layerData = rendered.result.current.layers[0].props.data as GeoJSON.Feature[];
		expect(layerData).toHaveLength(1);
		expect(layerData.some((f) => String(f.id) === String(CURRENT_ID))).toBe(false);
	});

	it("null bbox: setOn(true) still yields no layers (no bbox → no fetch)", () => {
		const rendered = renderHook(() => useNeighborOverlay(null, CURRENT_ID));
		mounted = rendered;

		act(() => {
			rendered.result.current.setOn(true);
		});

		expect(rendered.result.current.layers).toHaveLength(0);
	});

	it("getTooltip resolves a feature's name", () => {
		const rendered = renderHook(() => useNeighborOverlay(bbox, CURRENT_ID));
		mounted = rendered;

		// overlayTooltip gates on the overlay layer id (hardened in 617a9472), so the
		// picked info must carry layer.id === "geofence-overlay".
		const tip = rendered.result.current.getTooltip({
			layer: { id: "geofence-overlay" },
			object: { properties: { name: "neighbor" } },
		} as never);
		expect(tip).toEqual({ text: "neighbor" });
	});

	it("threads filters into the fetch URL", async () => {
		// Follow this file's existing fetch-stub pattern: useGeofencesByBboxMock
		// is a mock of the whole hook (not the underlying fetch), so assert on
		// what it was CALLED with rather than a URL string — this still proves
		// bbox/filters flow through unmangled to the wire-building hook, whose
		// own URL-building is covered by use-geo-features.test.tsx.
		useGeofencesByBboxMock.mockImplementation(
			(_bbox: unknown, enabled: boolean, filters?: { mode?: string; projects?: number[] }) => ({
				data: enabled ? FC : undefined,
				_filters: filters,
			}),
		);

		const rendered = renderHook(() =>
			useNeighborOverlay(bbox, CURRENT_ID, { mode: "pokemon", projects: [1, 2] }),
		);
		mounted = rendered;

		act(() => {
			rendered.result.current.setOn(true);
		});

		expect(useGeofencesByBboxMock).toHaveBeenCalledWith(bbox, true, {
			mode: "pokemon",
			projects: [1, 2],
		});
	});
});

describe("padBbox", () => {
	it("expands each axis by frac * span on both sides", () => {
		expect(padBbox([0, 0, 10, 10], 0.2)).toEqual([-2, -2, 12, 12]);
	});

	it("is null-safe", () => {
		expect(padBbox(null, 0.2)).toBe(null);
		expect(padBbox(undefined, 0.2)).toBe(null);
	});
});
