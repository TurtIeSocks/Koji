import type { StyleSpecification } from "maplibre-gl";

/** Build a minimal MapLibre style from a single XYZ raster tile template.
 *  Switching tile servers = calling this with a new url and swapping mapStyle. */
export function rasterStyle(tileUrl: string): StyleSpecification {
  return {
    version: 8,
    sources: {
      osm: { type: "raster", tiles: [tileUrl], tileSize: 256, attribution: "" },
    },
    layers: [{ id: "osm", type: "raster", source: "osm" }],
  };
}
