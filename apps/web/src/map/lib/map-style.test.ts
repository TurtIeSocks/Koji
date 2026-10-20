import { expect, test } from "vitest";
import { rasterStyle } from "@/map/lib/map-style";
import { DEFAULT_TILE_URL } from "@/lib/constants";

test("rasterStyle wraps an XYZ url in a single raster source + layer", () => {
  const style = rasterStyle("https://tile.example/{z}/{x}/{y}.png");
  expect(style.version).toBe(8);
  const src = style.sources.osm as { type: string; tiles: string[]; tileSize: number };
  expect(src.type).toBe("raster");
  expect(src.tiles).toEqual(["https://tile.example/{z}/{x}/{y}.png"]);
  expect(style.layers).toHaveLength(1);
  expect(style.layers[0]).toMatchObject({ type: "raster", source: "osm" });
});

test("rasterStyle expands Leaflet {s} subdomains and strips {r} (MapLibre can't)", () => {
  const style = rasterStyle("https://{s}.tiles.example/voyager/{z}/{x}/{y}{r}.png");
  const src = style.sources.osm as { tiles: string[] };
  expect(src.tiles).toEqual([
    "https://a.tiles.example/voyager/{z}/{x}/{y}.png",
    "https://b.tiles.example/voyager/{z}/{x}/{y}.png",
    "https://c.tiles.example/voyager/{z}/{x}/{y}.png",
    "https://d.tiles.example/voyager/{z}/{x}/{y}.png",
  ]);
});

test("rasterStyle on the default tile url loads OSM tiles with OSM attribution", () => {
  const style = rasterStyle(DEFAULT_TILE_URL);
  const src = style.sources.osm as { tiles: string[]; attribution: string };
  expect(src.tiles).toEqual(["https://tile.openstreetmap.org/{z}/{x}/{y}.png"]);
  expect(src.attribution).toBe(
    '© <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a> contributors',
  );
});

test("rasterStyle on a configured tile url overrides the default tiles and drops its attribution", () => {
  const style = rasterStyle("https://tiles.example/{z}/{x}/{y}.png");
  const src = style.sources.osm as { tiles: string[]; attribution: string };
  expect(src.tiles).toEqual(["https://tiles.example/{z}/{x}/{y}.png"]);
  expect(src.attribution).toBe("");
});
