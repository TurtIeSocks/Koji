import { useEffect, useState } from "react";
import { DEFAULT_TILE_URL } from "@/lib/constants";
import { getCachedAppConfig, loadAppConfig } from "@/lib/app-config";

const configured = (c: { tile_server?: string } | null) =>
  c?.tile_server?.trim() || null;

/**
 * The tile url a map should draw: an explicit `override` (the tile server
 * resource), else the tile server set in Koji's config (`TILE_SERVER`), else
 * the OSM default.
 */
export function useTileUrl(override?: string): string {
  const [fromConfig, setFromConfig] = useState(() => configured(getCachedAppConfig()));
  useEffect(() => {
    let alive = true;
    void loadAppConfig().then((c) => {
      if (alive) setFromConfig(configured(c));
    });
    return () => {
      alive = false;
    };
  }, []);
  return override ?? fromConfig ?? DEFAULT_TILE_URL;
}
