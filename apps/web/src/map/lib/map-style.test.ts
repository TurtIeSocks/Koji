import { expect, test } from "vitest";
import { rasterStyle } from "@/map/lib/map-style";

test("rasterStyle wraps an XYZ url in a single raster source + layer", () => {
  const style = rasterStyle("https://tile.example/{z}/{x}/{y}.png");
  expect(style.version).toBe(8);
  const src = style.sources.osm as { type: string; tiles: string[]; tileSize: number };
  expect(src.type).toBe("raster");
  expect(src.tiles).toEqual(["https://tile.example/{z}/{x}/{y}.png"]);
  expect(style.layers).toHaveLength(1);
  expect(style.layers[0]).toMatchObject({ type: "raster", source: "osm" });
});
