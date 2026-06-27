import { create } from "zustand";
import { persist } from "zustand/middleware";
import type { LayerId } from "@/map/stores/types";

const DEFAULT_VISIBILITY: Record<LayerId, boolean> = {
  gyms: false, pokestops: false, spawnpoints: false, stations: false,
  geofences: true, routes: true, s2: false,
};

export interface MapSettingsState {
  tileServerId: string;
  defaultLayerVisibility: Record<LayerId, boolean>;
  areaThresholds: { gym: number; pokestop: number; spawnpoint: number };
  markerRadius: number;
  setTileServerId: (id: string) => void;
  setMarkerRadius: (n: number) => void;
}

export const useMapSettingsStore = create<MapSettingsState>()(
  persist(
    (set) => ({
      tileServerId: "default",
      defaultLayerVisibility: DEFAULT_VISIBILITY,
      areaThresholds: { gym: 100, pokestop: 100, spawnpoint: 100 },
      markerRadius: 30,
      setTileServerId: (id) => set({ tileServerId: id }),
      setMarkerRadius: (n) => set({ markerRadius: n }),
    }),
    {
      name: "koji-map-settings",
      // persist EVERYTHING here is fine — this store holds only long-lived prefs.
      // transient camera/filters/selection live in OTHER stores and are never persisted.
    },
  ),
);
