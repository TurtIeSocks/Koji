export type LayerId =
  | "gyms"
  | "pokestops"
  | "spawnpoints"
  | "stations"
  | "geofences"
  | "routes"
  | "s2";

export type MarkerCategory = "gym" | "pokestop" | "spawnpoint" | "station" | "fort";

export interface ViewState {
  longitude: number;
  latitude: number;
  zoom: number;
  pitch: number;
  bearing: number;
}

/** deck bounds order: [minLng, minLat, maxLng, maxLat] */
export type Bounds = [number, number, number, number];

export interface Selection {
  kind: "marker" | "geofence" | "route" | null;
  id: string | null;
}

/** An S2 cell from `/s2/{level}`: the cell id plus its corner ring already
 *  transposed to deck `[lng, lat]` order (the server sends `[lat, lon]`). */
export interface S2Cell {
  id: string;
  ring: [number, number][];
}
