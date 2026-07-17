import { useEffect, useState } from "react";
import { loadConfig } from "@api";

// Module-level cache: every map shares one /internal/config fetch, and once it
// resolves new maps mount already-centered (no [0,0] flash on navigation).
let cached: [number, number] | null = null;
let inflight: Promise<[number, number]> | null = null;

function loadStartCenter(): Promise<[number, number]> {
  if (cached) return Promise.resolve(cached);
  if (!inflight) {
    inflight = loadConfig()
      .then((c): [number, number] => {
        cached = [c.start_lat ?? 0, c.start_lon ?? 0];
        return cached;
      })
      .catch((): [number, number] => [0, 0]); // unauth / offline → world center
  }
  return inflight;
}

// Warm the cache at import so the first map after login mounts centered.
void loadStartCenter();

/**
 * The map start center from server config (`START_LAT`/`START_LON`). Returns
 * `[0, 0]` until the config fetch resolves (or if it fails). Maps that show a
 * geometry still fit to it; this only sets the *empty* / pre-fit center.
 */
export function useStartCenter(): [number, number] {
  const [center, setCenter] = useState<[number, number]>(cached ?? [0, 0]);
  useEffect(() => {
    let alive = true;
    void loadStartCenter().then((c) => {
      if (alive) setCenter(c);
    });
    return () => {
      alive = false;
    };
  }, []);
  return center;
}
