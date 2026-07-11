import { describe, expect, it } from "vitest";
import {
	formatDuration,
	formatInt,
	formatMeters,
	formatPercent,
} from "./format";

describe("formatInt", () => {
	it("groups thousands", () => {
		// Grouping separator is locale-dependent; assert the digits stay in order.
		expect(formatInt(1234567)).toMatch(/1\D?234\D?567/);
	});
});

describe("formatPercent", () => {
	it("turns a 0..1 ratio into a percent", () => {
		expect(formatPercent(0.5)).toMatch(/50\s?%/);
		expect(formatPercent(1)).toMatch(/100\s?%/);
	});
});

describe("formatMeters", () => {
	it("shows meters under 1 km", () => {
		const s = formatMeters(500);
		expect(s).toMatch(/500/);
		expect(s).toMatch(/\bm\b/);
		expect(s).not.toMatch(/km/);
	});
	it("switches to km at/above 1 km", () => {
		expect(formatMeters(1500)).toMatch(/km/);
		expect(formatMeters(1500)).toMatch(/1\.5/);
	});
});

describe("formatDuration", () => {
	it("ms under 1 s", () => {
		expect(formatDuration(0.25)).toMatch(/250/);
		expect(formatDuration(0.25)).toMatch(/ms/);
	});
	it("seconds up to 90 s", () => {
		const s = formatDuration(45);
		expect(s).toMatch(/45/);
		expect(s).toMatch(/s/);
		expect(s).not.toMatch(/min/);
	});
	it("minutes beyond 90 s", () => {
		expect(formatDuration(120)).toMatch(/min/);
	});
});
