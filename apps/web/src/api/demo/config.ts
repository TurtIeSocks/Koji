import { DEFAULT_TILE_URL } from "@/lib/constants";
import type { ConfigResponse } from "../types";

/** Demo runtime config — the client-only stand-in for `GET /internal/config`.
 *  Only `start_lat`/`start_lon` are actually consumed today (see
 *  `useStartCenter`); `tile_server` mirrors the live default tile URL shape
 *  (an XYZ raster template fed through `rasterStyle`), and the plugin lists are
 *  empty because the demo world ships no plugins. Centered on Manhattan to
 *  match the seeded NYC-areas fixtures. */
export async function loadConfig(): Promise<ConfigResponse> {
  return {
    start_lat: 40.758,
    start_lon: -73.9855,
    tile_server: DEFAULT_TILE_URL,
    logged_in: true,
    dangerous: false,
    route_plugins: [],
    clustering_plugins: [],
    bootstrap_plugins: [],
  };
}
