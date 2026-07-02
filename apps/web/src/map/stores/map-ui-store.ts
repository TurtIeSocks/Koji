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
  /** The clicked geofence/route feature (for the "edit geometry" affordance). */
  selectedFeature: GeoJSON.Feature | null;
  /** Set while editing an EXISTING geofence → Save updates instead of creates. */
  editingGeofenceId: string | null;
  filters: Filters;
  toggleLayer: (id: LayerId) => void;
  setSelection: (sel: Selection) => void;
  setSelectedFeature: (f: GeoJSON.Feature | null) => void;
  setHover: (h: HoverInfo | null) => void;
  setS2Level: (n: number) => void;
  setDrawMode: (m: DrawMode) => void;
  setDraftFeatures: (fc: GeoJSON.FeatureCollection) => void;
  setSelectedFeatureIndexes: (ix: number[]) => void;
  /** Load an existing feature into the modify editor, tracking its id. */
  editFeature: (feature: GeoJSON.Feature) => void;
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
  selectedFeature: null,
  editingGeofenceId: null,
  filters: { lastSeen: 0, tth: "All" },
  toggleLayer: (id) =>
    set((s) => ({ layerVisibility: { ...s.layerVisibility, [id]: !s.layerVisibility[id] } })),
  setSelection: (sel) => set({ selection: sel }),
  setSelectedFeature: (f) => set({ selectedFeature: f }),
  setHover: (h) => set({ hoverInfo: h }),
  setS2Level: (n) => set({ s2Level: n }),
  setDrawMode: (m) => set({ drawMode: m }),
  setDraftFeatures: (fc) => set({ draftFeatures: fc }),
  setSelectedFeatureIndexes: (ix) => set({ selectedFeatureIndexes: ix }),
  editFeature: (feature) => {
    // koji serializes a geofence's id as the geojson top-level `feature.id`
    // (koji_geojson.rs), NOT properties.id — read that first. Getting this wrong
    // leaves editingGeofenceId null → Save creates a duplicate instead of
    // updating → the edit appears to revert.
    const rawId = feature.id ?? feature.properties?.id;
    set({
      draftFeatures: { type: "FeatureCollection", features: [feature] },
      drawMode: "modify",
      selectedFeatureIndexes: [0],
      editingGeofenceId: rawId != null ? String(rawId) : null,
      selection: { kind: null, id: null },
      selectedFeature: null,
    });
  },
  clearDraft: () =>
    set({ draftFeatures: EMPTY_FC, selectedFeatureIndexes: [], drawMode: "none", editingGeofenceId: null }),
  setLastSeen: (secs) => set((s) => ({ filters: { ...s.filters, lastSeen: secs } })),
  setTth: (t) => set((s) => ({ filters: { ...s.filters, tth: t } })),
}));
