import { describe, expect, it } from "vitest";
import { categoryToRouteMode, routeModeToCategory, routeModeMarkerCategories } from "./route-mode";

describe("categoryToRouteMode", () => {
	it("maps calc categories to route modes", () => {
		expect(categoryToRouteMode("spawnpoint")).toBe("pokemon");
		expect(categoryToRouteMode("pokestop")).toBe("quest");
		expect(categoryToRouteMode("gym")).toBe("fort");
		expect(categoryToRouteMode("unknown")).toBe("unset");
	});
});

describe("routeModeToCategory", () => {
	it("maps route modes to golbat categories (default pokestop)", () => {
		expect(routeModeToCategory("pokemon")).toBe("spawnpoint");
		expect(routeModeToCategory("quest")).toBe("pokestop");
		expect(routeModeToCategory("fort")).toBe("fort");
		expect(routeModeToCategory("unset")).toBe("pokestop");
		expect(routeModeToCategory(undefined)).toBe("pokestop");
	});
});

describe("routeModeMarkerCategories", () => {
	it("maps route modes to the marker categories to preview", () => {
		expect(routeModeMarkerCategories("pokemon")).toEqual(["spawnpoint"]);
		expect(routeModeMarkerCategories("quest")).toEqual(["pokestop"]);
		expect(routeModeMarkerCategories("fort")).toEqual(["gym", "station", "pokestop"]);
		expect(routeModeMarkerCategories("unset")).toEqual([]);
		expect(routeModeMarkerCategories(undefined)).toEqual([]);
	});
});
