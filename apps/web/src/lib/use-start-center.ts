import { useEffect, useState } from "react";
import { getCachedAppConfig, loadAppConfig } from "@/lib/app-config";

type Center = [number, number];

// unauth / offline → world center
const centerOf = (c: { start_lat?: number; start_lon?: number } | null): Center => [
  c?.start_lat ?? 0,
  c?.start_lon ?? 0,
];

/**
 * The map start center from server config (`START_LAT`/`START_LON`). Returns
 * `[0, 0]` until the config fetch resolves (or if it fails). Maps that show a
 * geometry still fit to it; this only sets the *empty* / pre-fit center.
 */
export function useStartCenter(): Center {
  const [center, setCenter] = useState<Center>(() => centerOf(getCachedAppConfig()));
  useEffect(() => {
    let alive = true;
    void loadAppConfig().then((c) => {
      if (alive) setCenter(centerOf(c));
    });
    return () => {
      alive = false;
    };
  }, []);
  return center;
}
