import { expect, test } from "vitest";
import {
	buildCalcBody,
	type CalcParams,
	featureToAreaFC,
	parseCalcResult,
	routeCoordsToClusters,
} from "@/map/lib/calc-request";

const P = (over: Partial<CalcParams> = {}): CalcParams => ({
	mode: "cluster",
	strategy: "radius",
	radius: 70,
	s2Level: 15,
	s2Size: 9,
	minPoints: 3,
	clusterMode: null,
	maxClusters: null,
	centerClusters: false,
	sortBy: null,
	tth: "All",
	...over,
});

const AREA = featureToAreaFC({
	type: "Feature",
	properties: {},
	geometry: { type: "Point", coordinates: [0, 0] },
});

test("routeCoordsToClusters transposes geojson [lon,lat] → koji [lat,lon]", () => {
	const line: GeoJSON.Feature = {
		type: "Feature",
		properties: {},
		geometry: {
			type: "LineString",
			coordinates: [
				[-122.3, 47.6],
				[-122.1, 47.4],
			],
		},
	};
	expect(routeCoordsToClusters(line)).toEqual([
		[47.6, -122.3],
		[47.4, -122.1],
	]);
	expect(routeCoordsToClusters(null)).toEqual([]);
});

test("cluster (radius) → backend route mode with radius clustering", () => {
	const body = buildCalcBody(P(), { area: AREA, category: "gym" });
	expect(body).toEqual({
		mode: "route",
		category: "gym",
		area: AREA,
		clustering: { calculationMode: "radius", minPoints: 3, radius: 70 },
	});
});

test("cluster (s2) sends s2Level/s2Size, not radius", () => {
	const body = buildCalcBody(P({ strategy: "s2", s2Level: 16, s2Size: 5 }), {
		area: AREA,
		category: "pokestop",
	});
	const clustering = body.clustering as Record<string, unknown>;
	expect(clustering.calculationMode).toBe("s2");
	expect(clustering.s2Level).toBe(16);
	expect(clustering.s2Size).toBe(5);
	expect(clustering).not.toHaveProperty("radius");
});

test("radius-only knobs ride only when set (clusterMode / maxClusters / centerClusters)", () => {
	const body = buildCalcBody(
		P({ clusterMode: "better", maxClusters: 12, centerClusters: true }),
		{ area: AREA, category: "pokestop" },
	);
	expect(body.clustering).toEqual({
		calculationMode: "radius",
		minPoints: 3,
		radius: 70,
		mode: "better",
		maxClusters: 12,
		centerClusters: true,
	});
	// maxClusters 0 → omitted (unlimited).
	const zero = buildCalcBody(P({ maxClusters: 0 }), { area: AREA });
	expect(zero.clustering).not.toHaveProperty("maxClusters");
});

test("dataFilter.lastSeen rides only when set (epoch seconds), composing with tth", () => {
	// Set → sent (server clusters only points updated after this instant).
	const body = buildCalcBody(P(), {
		area: AREA,
		category: "gym",
		lastSeen: 1_750_000_000,
	});
	expect(body.dataFilter).toEqual({ lastSeen: 1_750_000_000 });
	// 0 = no filter → omitted entirely (matches the marker preview's sentinel).
	expect(buildCalcBody(P(), { area: AREA, lastSeen: 0 })).not.toHaveProperty(
		"dataFilter",
	);
	expect(buildCalcBody(P(), { area: AREA })).not.toHaveProperty("dataFilter");
	// Composes with the spawnpoint tth filter in one dataFilter group.
	const both = buildCalcBody(P({ tth: "Known" }), {
		area: AREA,
		lastSeen: 600,
	});
	expect(both.dataFilter).toEqual({ tth: "Known", lastSeen: 600 });
});

test("bootstrap never sends dataFilter (backend gives it a fixed no-filter)", () => {
	const body = buildCalcBody(P({ mode: "bootstrap" }), {
		area: AREA,
		lastSeen: 600,
	});
	expect(body.mode).toBe("bootstrap");
	expect(body).not.toHaveProperty("dataFilter");
});

test("routing.sortBy rides only when set; unset omits routing (server default)", () => {
	expect(
		buildCalcBody(P({ sortBy: "geohash" }), { area: AREA }).routing,
	).toEqual({ sortBy: "geohash" });
	expect(buildCalcBody(P(), { area: AREA })).not.toHaveProperty("routing");
});

test("dataFilter.tth rides only when not All", () => {
	expect(
		(
			buildCalcBody(P({ tth: "Known" }), { area: AREA }).dataFilter as {
				tth: string;
			}
		).tth,
	).toBe("Known");
	expect(buildCalcBody(P({ tth: "All" }), { area: AREA })).not.toHaveProperty(
		"dataFilter",
	);
});

test("bootstrap uses the bootstrap group", () => {
	expect(
		buildCalcBody(P({ mode: "bootstrap", radius: 90 }), {
			area: AREA,
			category: "pokestop",
		}),
	).toEqual({
		mode: "bootstrap",
		category: "pokestop",
		area: AREA,
		bootstrap: { calculationMode: "radius", radius: 90 },
	});
	const s2 = buildCalcBody(
		P({ mode: "bootstrap", strategy: "s2", s2Level: 14, s2Size: 7 }),
		{ area: AREA },
	);
	expect(s2.bootstrap).toEqual({
		calculationMode: "s2",
		radius: 70,
		s2Level: 14,
		s2Size: 7,
	});
});

test("parseCalcResult pulls the FC + stats from a succeeded record", () => {
	const fc: GeoJSON.FeatureCollection = {
		type: "FeatureCollection",
		features: [],
	};
	const r = parseCalcResult({
		result: { data: fc, stats: { total_clusters: 5 } },
	});
	expect(r.fc).toBe(fc);
	expect(r.stats).toEqual({ total_clusters: 5 });
	expect(parseCalcResult({ result: null }).fc).toBeNull();
	expect(parseCalcResult(null).fc).toBeNull();
});
