import { describe, expect, it } from "vitest";
import { toEpochSeconds, toLocalInput } from "./use-last-seen";

describe("toEpochSeconds", () => {
	it("empty string → 0 (the backend's no-filter sentinel)", () => {
		expect(toEpochSeconds("")).toBe(0);
	});

	it("unparseable → 0", () => {
		expect(toEpochSeconds("not-a-date")).toBe(0);
	});

	it("a datetime-local string → its local epoch seconds", () => {
		// Parsed as local time, then floored to whole seconds.
		const s = "2026-06-10T09:30";
		expect(toEpochSeconds(s)).toBe(Math.floor(new Date(s).getTime() / 1000));
	});
});

describe("toLocalInput", () => {
	it("round-trips through toEpochSeconds at minute precision", () => {
		const d = new Date(2026, 5, 10, 9, 30, 0, 0); // local
		const s = toLocalInput(d);
		expect(s).toBe("2026-06-10T09:30");
		expect(toEpochSeconds(s)).toBe(Math.floor(d.getTime() / 1000));
	});

	it("zero-pads month/day/hour/minute", () => {
		expect(toLocalInput(new Date(2026, 0, 3, 4, 5, 0, 0))).toBe(
			"2026-01-03T04:05",
		);
	});
});
