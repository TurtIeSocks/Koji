import type { StyleSpecification } from "maplibre-gl";
import { DEFAULT_TILE_ATTRIBUTION, DEFAULT_TILE_URL } from "@/lib/constants";

/** Build a minimal MapLibre style from an XYZ raster tile template.
 *  Switching tile servers = calling this with a new url and swapping mapStyle.
 *
 *  MapLibre raster sources do NOT expand Leaflet-style `{s}` (subdomain) or
 *  `{r}` (retina) placeholders, which Koji's tile URLs use. So strip `{r}` and
 *  expand `{s}` into one tile URL per subdomain (a/b/c/d).
 *
 *  Only the default (OSM) url carries an attribution; a configured server's
 *  credit is its own to give. */
export function rasterStyle(tileUrl: string): StyleSpecification {
  const noRetina = tileUrl.replace(/\{r\}/g, "");
  const tiles = noRetina.includes("{s}")
    ? ["a", "b", "c", "d"].map((s) => noRetina.replace(/\{s\}/g, s))
    : [noRetina];
  return {
    version: 8,
    sources: {
      osm: {
        type: "raster",
        tiles,
        tileSize: 256,
        attribution: tileUrl === DEFAULT_TILE_URL ? DEFAULT_TILE_ATTRIBUTION : "",
      },
    },
    layers: [{ id: "osm", type: "raster", source: "osm" }],
  };
}
