import { create } from "zustand";
import type { LayerId, Selection } from "@/map/stores/types";

interface HoverInfo { x: number; y: number; id: string | null; }

export interface MapUIState {
  layerVisibility: Record<LayerId, boolean>;
  selection: Selection;
  hoverInfo: HoverInfo | null;
  s2Level: number;
  toggleLayer: (id: LayerId) => void;
  setSelection: (sel: Selection) => void;
  setHover: (h: HoverInfo | null) => void;
  setS2Level: (n: number) => void;
}

const INITIAL_VISIBILITY: Record<LayerId, boolean> = {
  gyms: false, pokestops: false, spawnpoints: false, stations: false,
  geofences: true, routes: true, s2: false,
};

export const useMapUIStore = create<MapUIState>()((set) => ({
  layerVisibility: INITIAL_VISIBILITY,
  selection: { kind: null, id: null },
  hoverInfo: null,
  s2Level: 15,
  toggleLayer: (id) =>
    set((s) => ({ layerVisibility: { ...s.layerVisibility, [id]: !s.layerVisibility[id] } })),
  setSelection: (sel) => set({ selection: sel }),
  setHover: (h) => set({ hoverInfo: h }),
  setS2Level: (n) => set({ s2Level: n }),
}));
