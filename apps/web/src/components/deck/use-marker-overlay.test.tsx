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
const { useMarkersMock } = vi.hoisted(() => ({ useMarkersMock: vi.fn() }));

vi.mock("@/map/data/use-markers", () => ({ useMarkers: useMarkersMock }));

import { useMarkerOverlay } from "./use-marker-overlay";

// `@testing-library/react` isn't a dependency of this project — hand-roll the
// minimal `renderHook` shape (result ref + act) backed by `react-dom/client`
// + React 19's own `act`, matching `use-calc.test.tsx`.
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

const POINTS_BY_CATEGORY: Record<string, [number, number][]> = {
	gym: [[10, 20]],
	pokestop: [[11, 21]],
	spawnpoint: [[12, 22]],
	station: [[13, 23]],
};

let mounted: { unmount: () => void } | null = null;
afterEach(() => {
	mounted?.unmount();
	mounted = null;
});

describe("useMarkerOverlay", () => {
	beforeEach(() => {
		useMarkersMock.mockReset();
		// Mirrors the real useMarkers contract: `data` is undefined until the
		// (mocked) query is enabled and "resolves".
		useMarkersMock.mockImplementation(
			(category: string, _area: unknown, _bounds: unknown, _lastSeen: number, enabled: boolean) => ({
				data: enabled ? (POINTS_BY_CATEGORY[category] ?? []) : undefined,
			}),
		);
	});

	it("defaults off: no marker layers, label matches the pokemon mode", () => {
		const rendered = renderHook(() => useMarkerOverlay("pokemon", poly));
		mounted = rendered;

		expect(rendered.result.current.on).toBe(false);
		expect(rendered.result.current.markerLayers).toHaveLength(0);
		expect(rendered.result.current.label).toBe("Spawnpoints");
	});

	it("setOn(true) for pokemon mode builds a spawnpoint-tagged layer", () => {
		const rendered = renderHook(() => useMarkerOverlay("pokemon", poly));
		mounted = rendered;

		act(() => {
			rendered.result.current.setOn(true);
		});

		expect(rendered.result.current.on).toBe(true);
		expect(rendered.result.current.markerLayers.length).toBeGreaterThanOrEqual(1);
		expect(
			rendered.result.current.markerLayers.some((l) => l.id.includes("spawnpoint")),
		).toBe(true);
		// Only the pokemon-mode category should have fetched/rendered.
		expect(
			rendered.result.current.markerLayers.every(
				(l) => l.id.includes("spawnpoint"),
			),
		).toBe(true);
	});

	it("quest mode labels 'Pokestops' and toggling on tags a pokestop layer", () => {
		const rendered = renderHook(() => useMarkerOverlay("quest", poly));
		mounted = rendered;

		expect(rendered.result.current.label).toBe("Pokestops");

		act(() => {
			rendered.result.current.setOn(true);
		});

		expect(
			rendered.result.current.markerLayers.some((l) => l.id.includes("pokestop")),
		).toBe(true);
	});

	it("fort mode labels 'Forts' and an unset/unknown mode labels 'Markers'", () => {
		const fort = renderHook(() => useMarkerOverlay("fort", poly));
		mounted = fort;
		expect(fort.result.current.label).toBe("Forts");
		fort.unmount();

		const other = renderHook(() => useMarkerOverlay(undefined, poly));
		mounted = other;
		expect(other.result.current.label).toBe("Markers");
	});

	it("`available` is true for a mode with categories, false for unset/unknown", () => {
		const pokemon = renderHook(() => useMarkerOverlay("pokemon", poly));
		mounted = pokemon;
		expect(pokemon.result.current.available).toBe(true);
		pokemon.unmount();

		const fort = renderHook(() => useMarkerOverlay("fort", poly));
		mounted = fort;
		expect(fort.result.current.available).toBe(true);
		fort.unmount();

		const unset = renderHook(() => useMarkerOverlay("unset", poly));
		mounted = unset;
		expect(unset.result.current.available).toBe(false);
		unset.unmount();

		const undef = renderHook(() => useMarkerOverlay(undefined, poly));
		mounted = undef;
		expect(undef.result.current.available).toBe(false);
	});

	it("null/undefined area still renders (fixed hook order) with everything off", () => {
		const rendered = renderHook(() => useMarkerOverlay("pokemon", null));
		mounted = rendered;

		expect(rendered.result.current.on).toBe(false);
		expect(rendered.result.current.markerLayers).toHaveLength(0);
		// All four categories were still called, just disabled — assert the call
		// count instead of relying on hook-order internals we can't observe here.
		expect(useMarkersMock).toHaveBeenCalledTimes(4);
	});

	it("alwaysFetch populates `data` while the toggle stays off", () => {
		const rendered = renderHook(() =>
			useMarkerOverlay("pokemon", poly, { alwaysFetch: true }),
		);
		mounted = rendered;

		expect(rendered.result.current.on).toBe(false);
		expect(rendered.result.current.markerLayers).toHaveLength(0);
		expect(rendered.result.current.data.spawnpoint).toEqual([[12, 22]]);
		// Non-pokemon categories stay disabled even with alwaysFetch.
		expect(rendered.result.current.data.pokestop).toBeUndefined();
	});

	it("null area disables every fetch even with alwaysFetch (no unbounded world query)", () => {
		const rendered = renderHook(() =>
			useMarkerOverlay("pokemon", null, { alwaysFetch: true }),
		);
		mounted = rendered;

		expect(rendered.result.current.data).toEqual({});
		for (const call of useMarkersMock.mock.calls) {
			const enabled = call[4];
			expect(enabled).toBe(false);
		}
	});

	it("calls useMarkers exactly 4 times per render (fixed hook order)", () => {
		const rendered = renderHook(() => useMarkerOverlay("fort", poly));
		mounted = rendered;
		expect(useMarkersMock).toHaveBeenCalledTimes(4);

		act(() => {
			rendered.result.current.setOn(true);
		});
		// One more render → 4 more calls (8 total), never a variable count.
		expect(useMarkersMock).toHaveBeenCalledTimes(8);
	});
});
