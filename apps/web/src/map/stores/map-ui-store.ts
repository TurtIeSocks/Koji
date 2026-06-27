import { create } from "zustand";
import type { LayerId, Selection } from "@/map/stores/types";
import type { DrawMode } from "@/map/lib/edit-modes";

interface HoverInfo { x: number; y: number; id: string | null; }

interface Filters {
  lastSeen: number;
  tth: "All" | "Known" | "Unknown";
}

export interface MapUIState {
  layerVisibility: Record<LayerId, boolean>;
  selection: Selection;
  hoverInfo: HoverInfo | null;
  s2Level: number;
  drawMode: DrawMode;
  draftFeatures: GeoJSON.FeatureCollection;
  selectedFeatureIndexes: number[];
  filters: Filters;
  toggleLayer: (id: LayerId) => void;
  setSelection: (sel: Selection) => void;
  setHover: (h: HoverInfo | null) => void;
  setS2Level: (n: number) => void;
  setDrawMode: (m: DrawMode) => void;
  setDraftFeatures: (fc: GeoJSON.FeatureCollection) => void;
  setSelectedFeatureIndexes: (ix: number[]) => void;
  clearDraft: () => void;
  setLastSeen: (secs: number) => void;
  setTth: (t: "All" | "Known" | "Unknown") => void;
}

const INITIAL_VISIBILITY: Record<LayerId, boolean> = {
  gyms: false, pokestops: false, spawnpoints: false, stations: false,
  geofences: true, routes: true, s2: false,
};

const EMPTY_FC: GeoJSON.FeatureCollection = { type: "FeatureCollection", features: [] };

export const useMapUIStore = create<MapUIState>()((set) => ({
  layerVisibility: INITIAL_VISIBILITY,
  selection: { kind: null, id: null },
  hoverInfo: null,
  s2Level: 15,
  drawMode: "none",
  draftFeatures: EMPTY_FC,
  selectedFeatureIndexes: [],
  filters: { lastSeen: 0, tth: "All" },
  toggleLayer: (id) =>
    set((s) => ({ layerVisibility: { ...s.layerVisibility, [id]: !s.layerVisibility[id] } })),
  setSelection: (sel) => set({ selection: sel }),
  setHover: (h) => set({ hoverInfo: h }),
  setS2Level: (n) => set({ s2Level: n }),
  setDrawMode: (m) => set({ drawMode: m }),
  setDraftFeatures: (fc) => set({ draftFeatures: fc }),
  setSelectedFeatureIndexes: (ix) => set({ selectedFeatureIndexes: ix }),
  clearDraft: () => set({ draftFeatures: EMPTY_FC, selectedFeatureIndexes: [], drawMode: "none" }),
  setLastSeen: (secs) => set((s) => ({ filters: { ...s.filters, lastSeen: secs } })),
  setTth: (t) => set((s) => ({ filters: { ...s.filters, tth: t } })),
}));
