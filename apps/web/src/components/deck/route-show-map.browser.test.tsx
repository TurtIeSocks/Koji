// Tailwind's utility classes (position/z-index) are only compiled into a real
// stylesheet when this global CSS entrypoint is imported — the "browser"
// vitest project has no shared setupFiles, so the toggle button's `.click()`
// wouldn't land over the deck.gl canvas without it (established in the
// DeckMap expand-button test).
import "@/index.css";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { render } from "vitest-browser-react";
import { RecordContextProvider, testDataProvider, type DataProvider } from "shadmin-core";
import { AdminContext } from "@/components/admin";

// Hoisted so both the vi.mock factories and the tests can drive them.
const { submitMock, getJobMock } = vi.hoisted(() => ({
	submitMock: vi.fn(),
	getJobMock: vi.fn(),
}));

// Real `useCalc` (x2) runs in this component — stub the network layer under it
// so the auto-computed route-stats job resolves deterministically without
// hitting the network (mirrors use-calc.test.tsx / map-playground.browser.test.tsx).
vi.mock("@/api/live/calc", async (importActual) => ({
	...(await importActual<typeof import("@/api/live/calc")>()),
	submitCalc: submitMock,
	getJob: getJobMock,
}));
// Spread the real module so `@api`'s live realtime transport (and the other
// realtime exports) stay resolvable in browser/ESM mode; only useSubscribe
// is stubbed.
vi.mock("@/components/realtime", async (importActual) => ({
	...(await importActual<typeof import("@/components/realtime")>()),
	useSubscribe: vi.fn(),
}));

const SPAWNPOINTS: [number, number][] = [
	[4.905, 51.905],
	[4.915, 51.915],
];

vi.mock("@/map/data/use-markers", () => ({
	useMarkers: vi.fn(
		(
			category: string,
			_area: unknown,
			_bounds: unknown,
			_lastSeen: number,
			enabled: boolean,
		) => ({
			data: !enabled ? undefined : category === "spawnpoint" ? SPAWNPOINTS : [],
		}),
	),
}));

import { RouteShowMap } from "./route-show-map";

const fencePoly: GeoJSON.Polygon = {
	type: "Polygon",
	coordinates: [
		[
			[4.8, 51.8],
			[5.0, 51.8],
			[5.0, 52.0],
			[4.8, 52.0],
			[4.8, 51.8],
		],
	],
};

const routeRecord = {
	id: 1,
	geofence_id: 1,
	mode: "pokemon",
	geometry: {
		type: "MultiPoint",
		coordinates: [
			[4.9, 51.9],
			[4.91, 51.91],
		],
	} as GeoJSON.MultiPoint,
};

function renderRouteShowMap() {
	const getOneSpy: DataProvider["getOne"] = vi.fn(async (_resource, params) => ({
		data: { id: params.id, geometry: fencePoly },
	})) as DataProvider["getOne"];
	const screen = render(
		<AdminContext dataProvider={testDataProvider({ getOne: getOneSpy })}>
			<RecordContextProvider value={routeRecord}>
				<RouteShowMap />
			</RecordContextProvider>
		</AdminContext>,
	);
	return { screen, getOneSpy };
}

beforeEach(() => {
	submitMock.mockReset().mockResolvedValue("77");
	getJobMock.mockReset().mockResolvedValue({
		id: 77,
		status: "Succeeded",
		progress: 1,
		phase: null,
		result: {
			data: { type: "FeatureCollection", features: [] },
			stats: { total_clusters: 1, total_points: 2, points_covered: 2 },
		},
	});
	// CALC_PERSIST_KEY (`koji.map.calc`) is a shared localStorage key — a prior
	// test/file could have left a non-default radius behind.
	localStorage.removeItem("koji.map.calc");
});

describe("RouteShowMap", () => {
	it("renders the map + marker toggle + stats panel, and the toggle flips its label on click", async () => {
		const { screen, getOneSpy } = renderRouteShowMap();

		// The real deck-map container renders (not an empty placeholder).
		await expect.element(screen.getByTestId("deck-map")).toBeInTheDocument();
		await vi.waitFor(() => {
			expect(getOneSpy).toHaveBeenCalledWith(
				"geofence",
				expect.objectContaining({ id: 1 }),
			);
		});

		// Mode-driven marker toggle, default OFF ("pokemon" -> "Spawnpoints").
		const toggle = screen.getByRole("button", { name: "Show Spawnpoints" });
		await expect.element(toggle).toBeInTheDocument();

		// RouteStatsPanel root renders once the debounced routeStats job resolves.
		await expect
			.element(screen.getByText("Route stats"), { timeout: 3000 })
			.toBeInTheDocument();

		await toggle.click();
		await expect
			.element(screen.getByRole("button", { name: "Hide Spawnpoints" }))
			.toBeInTheDocument();
	});
});
