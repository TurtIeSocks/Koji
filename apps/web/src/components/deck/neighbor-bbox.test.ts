import { describe, expect, it } from "vitest";
import { centerBbox, NEIGHBOR_MIN_ZOOM, neighborBbox, roundBbox } from "./neighbor-bbox";

const VIEW_BOUNDS: [number, number, number, number] = [10, 20, 12, 22];
const START_LON = -73;
const START_LAT = 40;

describe("neighborBbox", () => {
  it("suppresses the fetch just below the zoom floor", () => {
    const view = { bounds: VIEW_BOUNDS, zoom: NEIGHBOR_MIN_ZOOM - 0.1 };
    expect(neighborBbox(view, null, START_LON, START_LAT)).toBeNull();
  });

  it("fetches at exactly the zoom floor", () => {
    const view = { bounds: VIEW_BOUNDS, zoom: NEIGHBOR_MIN_ZOOM };
    expect(neighborBbox(view, null, START_LON, START_LAT)).toEqual(VIEW_BOUNDS);
  });

  it("fetches above the zoom floor", () => {
    const view = { bounds: VIEW_BOUNDS, zoom: NEIGHBOR_MIN_ZOOM + 0.1 };
    expect(neighborBbox(view, null, START_LON, START_LAT)).toEqual(VIEW_BOUNDS);
  });

  it("falls back to the padded geometry bbox before any camera interaction", () => {
    const geometry: GeoJSON.Polygon = {
      type: "Polygon",
      coordinates: [[[10, 20], [12, 20], [12, 22], [10, 22], [10, 20]]],
    };
    const result = neighborBbox(null, geometry, START_LON, START_LAT);
    // Raw geometry bounds are [10, 20, 12, 22] — padding by 0.2 (20% of each
    // axis span) must expand it, not just pass it through.
    expect(result).not.toEqual([10, 20, 12, 22]);
    expect(result).toEqual([9.6, 19.6, 12.4, 22.4]);
  });

  it("falls back to a box around the start center with no geometry and no camera interaction", () => {
    expect(neighborBbox(null, null, START_LON, START_LAT)).toEqual(
      centerBbox(START_LON, START_LAT),
    );
  });

  it("centerBbox shape: fixed pad around the point", () => {
    expect(centerBbox(START_LON, START_LAT)).toEqual([
      START_LON - 0.15,
      START_LAT - 0.1,
      START_LON + 0.15,
      START_LAT + 0.1,
    ]);
  });
});

describe("roundBbox", () => {
  it("rounds each coordinate to 4dp", () => {
    expect(roundBbox([1.123456, 2.987654, -3.00001, 4.00009])).toEqual([
      1.1235, 2.9877, -3.0, 4.0001,
    ]);
  });
});
