import { describe, expect, it } from "vitest";
import { parseGeoJsonText, parseLatLonText, parseSourceText } from "./geojson-source";

describe("parseGeoJsonText", () => {
  it("accepts a FeatureCollection and returns its features", () => {
    const out = parseGeoJsonText(
      JSON.stringify({
        type: "FeatureCollection",
        features: [
          {
            type: "Feature",
            geometry: { type: "Polygon", coordinates: [] },
            properties: { name: "A" },
          },
        ],
      }),
    );
    expect("features" in out && out.features).toHaveLength(1);
  });

  it("wraps a bare Feature into a one-feature collection", () => {
    const out = parseGeoJsonText(
      JSON.stringify({
        type: "Feature",
        geometry: { type: "Point", coordinates: [0, 0] },
        properties: {},
      }),
    );
    expect("features" in out && out.features).toHaveLength(1);
  });

  it("returns a typed error for malformed JSON (not a silent empty)", () => {
    const out = parseGeoJsonText("{ not json");
    expect("error" in out && out.error).toMatch(/json/i);
  });

  it("returns a typed error when there are no features", () => {
    const out = parseGeoJsonText(
      JSON.stringify({ type: "FeatureCollection", features: [] }),
    );
    expect("error" in out && out.error).toMatch(/no features/i);
  });

  it("returns a typed error for a non-GeoJSON object", () => {
    const out = parseGeoJsonText(JSON.stringify({ hello: "world" }));
    expect("error" in out && out.error).toMatch(/feature/i);
  });
});

const nlList = "52.37,4.89\n52.38,4.90\n52.36,4.91";

describe("parseLatLonText", () => {
  it("parses a newline-delimited lat,lon list into ONE closed Polygon feature", () => {
    const r = parseLatLonText(nlList);
    if ("error" in r) throw new Error(r.error);
    expect(r.features).toHaveLength(1);
    const geom = r.features[0].geometry as GeoJSON.Polygon;
    expect(geom.type).toBe("Polygon");
    // THE ORDER TRAP (spec §2.1): input is lat,lon → GeoJSON is [lon, lat].
    // 52.37 is a latitude (Amsterdam); if it lands in position 0 the flip is missing.
    expect(geom.coordinates[0][0]).toEqual([4.89, 52.37]);
    // Ring closed: last === first, 4 positions from 3 points.
    expect(geom.coordinates[0]).toHaveLength(4);
    expect(geom.coordinates[0][3]).toEqual(geom.coordinates[0][0]);
  });

  it("parses v1's comma-separated 'lat lon' pair form", () => {
    // v1 heuristic (main:server/model/src/api/text.rs:8-15): first whitespace
    // token parses as a float → pairs are "lat lon", separated by commas.
    const r = parseLatLonText("52.37 4.89, 52.38 4.90, 52.36 4.91");
    if ("error" in r) throw new Error(r.error);
    const geom = r.features[0].geometry as GeoJSON.Polygon;
    expect(geom.coordinates[0][0]).toEqual([4.89, 52.37]);
    expect(geom.coordinates[0]).toHaveLength(4);
  });

  it("does not double-close an already-closed ring", () => {
    const r = parseLatLonText(`${nlList}\n52.37,4.89`);
    if ("error" in r) throw new Error(r.error);
    expect((r.features[0].geometry as GeoJSON.Polygon).coordinates[0]).toHaveLength(4);
  });

  it("tolerates blank lines", () => {
    const r = parseLatLonText("52.37,4.89\n\n52.38,4.90\n52.36,4.91\n");
    if ("error" in r) throw new Error(r.error);
    expect((r.features[0].geometry as GeoJSON.Polygon).coordinates[0]).toHaveLength(4);
  });

  it("rejects fewer than 3 distinct points", () => {
    expect(parseLatLonText("52.37,4.89\n52.38,4.90")).toHaveProperty("error");
    // 2 distinct + explicit closure is still 2 distinct.
    expect(parseLatLonText("52.37,4.89\n52.38,4.90\n52.37,4.89")).toHaveProperty("error");
  });

  it("accepts and drops tokens beyond lat,lon (alt columns in scanner exports)", () => {
    // lat,lon,alt is a common export shape (dragonite/poracle). Extra tokens
    // are deliberately ignored — v1 read only the first two fields too.
    const nl = parseLatLonText("52.37,4.89,120.5\n52.38,4.90,121\n52.36,4.91,119");
    if ("error" in nl) throw new Error(nl.error);
    expect((nl.features[0].geometry as GeoJSON.Polygon).coordinates[0][0]).toEqual([4.89, 52.37]);
    const sp = parseLatLonText("52.37 4.89 120.5, 52.38 4.90 121, 52.36 4.91 119");
    if ("error" in sp) throw new Error(sp.error);
    expect((sp.features[0].geometry as GeoJSON.Polygon).coordinates[0][0]).toEqual([4.89, 52.37]);
  });

  it("rejects out-of-range and garbage pairs with a readable error", () => {
    const bad = parseLatLonText("52.37,4.89\n999,4.90\n52.36,4.91");
    expect(bad).toHaveProperty("error");
    expect((bad as { error: string }).error).toContain("999");
    expect(parseLatLonText("not a list at all")).toHaveProperty("error");
  });
});

describe("parseSourceText", () => {
  it("routes JSON-looking input to the GeoJSON parser", () => {
    const fc = `{"type":"FeatureCollection","features":[{"type":"Feature","geometry":{"type":"Point","coordinates":[0,0]},"properties":{}}]}`;
    expect(parseSourceText(fc)).toEqual(parseGeoJsonText(fc));
    // Malformed JSON reports the JSON error, not a lat/lon error.
    expect((parseSourceText("{oops") as { error: string }).error).toMatch(/JSON/i);
  });

  it("routes bare coordinate text to the lat/lon parser", () => {
    const r = parseSourceText(nlList);
    if ("error" in r) throw new Error(r.error);
    expect((r.features[0].geometry as GeoJSON.Polygon).type).toBe("Polygon");
  });
});
