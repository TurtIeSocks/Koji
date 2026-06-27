import type { Bounds } from "@/map/stores/types";

/** Koji marker pairs are [lat, lon]; deck/GeoJSON want [lng, lat]. */
export function fromKojiLatLon(pair: [number, number]): [number, number] {
  return [pair[1], pair[0]];
}

/** Flatten Koji [lat,lon] pairs into a [lng,lat,...] Float32Array for deck binary accessors. */
export function packMarkers(points: [number, number][]): Float32Array {
  const out = new Float32Array(points.length * 2);
  for (let i = 0; i < points.length; i++) {
    out[i * 2] = points[i][1]; // lng
    out[i * 2 + 1] = points[i][0]; // lat
  }
  return out;
}

export function boundsToBboxArg(
  b: Bounds,
): { min_lat: number; min_lon: number; max_lat: number; max_lon: number } {
  return { min_lon: b[0], min_lat: b[1], max_lon: b[2], max_lat: b[3] };
}
