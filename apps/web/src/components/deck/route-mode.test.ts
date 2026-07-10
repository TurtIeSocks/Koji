import { describe, expect, it } from "vitest";
import { categoryToRouteMode, routeModeToCategory } from "./route-mode";

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
