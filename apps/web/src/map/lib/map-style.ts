import type { StyleSpecification } from "maplibre-gl";

/** Build a minimal MapLibre style from an XYZ raster tile template.
 *  Switching tile servers = calling this with a new url and swapping mapStyle.
 *
 *  MapLibre raster sources do NOT expand Leaflet-style `{s}` (subdomain) or
 *  `{r}` (retina) placeholders, which Koji's tile URLs use. So strip `{r}` and
 *  expand `{s}` into one tile URL per subdomain (a/b/c/d). */
export function rasterStyle(tileUrl: string): StyleSpecification {
  const noRetina = tileUrl.replace(/\{r\}/g, "");
  const tiles = noRetina.includes("{s}")
    ? ["a", "b", "c", "d"].map((s) => noRetina.replace(/\{s\}/g, s))
    : [noRetina];
  return {
    version: 8,
    sources: {
      osm: { type: "raster", tiles, tileSize: 256, attribution: "" },
    },
    layers: [{ id: "osm", type: "raster", source: "osm" }],
  };
}
