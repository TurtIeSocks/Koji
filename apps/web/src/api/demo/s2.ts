import { boundsToBboxArg, fromKojiLatLon } from "@/map/lib/coords";
import type { Bounds, S2Cell } from "@/map/stores/types";
import { getWasm } from "./wasm";

/** The wasm `s2_cells` shape: numeric id + a Koji `[lat, lon]` corner ring. */
interface RawS2Cell {
  id: number | string;
  coords: [number, number][];
}

/** Demo mirror of live `POST /api/v2/s2/{level}` — computes the covering cells
 *  entirely in-browser via `@koji-wasm`'s `s2_cells`, which is the same
 *  `koji_core::s2::get_cells` the server runs. Returns cell corner rings as
 *  deck-friendly `[lng, lat]` (the raw wasm coords are Koji `[lat, lon]`). */
export async function fetchS2Cells(level: number, bounds: Bounds): Promise<S2Cell[]> {
  const wasm = await getWasm();
  const b = boundsToBboxArg(bounds);
  const raw = wasm.s2_cells(level, b.min_lat, b.min_lon, b.max_lat, b.max_lon) as RawS2Cell[];
  return (raw ?? []).map((c) => ({
    id: String(c.id),
    ring: (c.coords ?? []).map(fromKojiLatLon),
  }));
}
