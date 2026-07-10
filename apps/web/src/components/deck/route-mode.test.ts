import { describe, expect, it } from "vitest";
import { categoryToRouteMode } from "./route-mode";

describe("categoryToRouteMode", () => {
	it("maps calc categories to route modes", () => {
		expect(categoryToRouteMode("spawnpoint")).toBe("pokemon");
		expect(categoryToRouteMode("pokestop")).toBe("quest");
		expect(categoryToRouteMode("gym")).toBe("fort");
		expect(categoryToRouteMode("unknown")).toBe("unset");
	});
});
