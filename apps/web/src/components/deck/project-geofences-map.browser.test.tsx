// Tailwind's utility classes (position/z-index) are only compiled into a real
// stylesheet when this global CSS entrypoint is imported — the "browser"
// vitest project has no shared setupFiles, so DeckMap's layout wouldn't be
// correct without it (established in the DeckMap expand-button test).
import "@/index.css";
import { describe, expect, it, vi } from "vitest";
import { render } from "vitest-browser-react";
import { testDataProvider } from "shadmin-core";
import { AdminContext } from "@/components/admin";

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

const TWO_FEATURE_FC: GeoJSON.FeatureCollection = {
	type: "FeatureCollection",
	features: [
		{ type: "Feature", id: 1, properties: { id: 1, name: "fence one" }, geometry: poly },
		{ type: "Feature", id: 2, properties: { id: 2, name: "fence two" }, geometry: poly },
	],
};

const { useGeofencesByIdsMock } = vi.hoisted(() => ({
	useGeofencesByIdsMock: vi.fn(),
}));

vi.mock("@/map/data/use-geo-features", () => ({
	useGeofencesByIds: useGeofencesByIdsMock,
}));

import { ProjectGeofencesMap } from "./project-geofences-map";

function renderMap(ids: (number | string)[]) {
	return render(
		<AdminContext dataProvider={testDataProvider()}>
			<ProjectGeofencesMap ids={ids} />
		</AdminContext>,
	);
}

describe("ProjectGeofencesMap", () => {
	it("renders a deck-map fit to the member features when ids are given", async () => {
		useGeofencesByIdsMock.mockReturnValue({ data: TWO_FEATURE_FC });

		const screen = renderMap([1, 2]);

		await expect.element(screen.getByTestId("deck-map")).toBeInTheDocument();
		expect(screen.getByText("No geofences").elements()).toHaveLength(0);
	});

	it("renders the empty placeholder (no deck-map) when ids is empty", async () => {
		useGeofencesByIdsMock.mockReturnValue({ data: undefined });

		const screen = renderMap([]);

		await expect.element(screen.getByText("No geofences")).toBeInTheDocument();
		expect(screen.getByTestId("deck-map").elements()).toHaveLength(0);
	});
});
