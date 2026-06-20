import { describe, expect, it } from "vitest";
import { parseGeoJsonText } from "./geojson-source";

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
