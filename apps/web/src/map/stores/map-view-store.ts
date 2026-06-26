import { create } from "zustand";
import { subscribeWithSelector } from "zustand/middleware";
import type { Bounds, ViewState } from "@/map/stores/types";

const DEFAULT_VIEW: ViewState = { longitude: 0, latitude: 0, zoom: 2, pitch: 0, bearing: 0 };
const DEFAULT_BOUNDS: Bounds = [-180, -85, 180, 85];

export interface MapViewState {
  /** Hot path — written ~60fps. Read transiently (getState / subscribe in effect), NEVER via useStore in render. */
  liveViewState: ViewState;
  /** Throttled snapshot — the ONLY camera value safe to subscribe to in render. */
  settledViewState: ViewState;
  settledBounds: Bounds;
  setLive: (v: ViewState, bounds: Bounds) => void;
  flushSettle: () => void;
}

export function createMapViewStore(throttleMs = 200) {
  let pending: { v: ViewState; b: Bounds } | null = null;
  let timer: ReturnType<typeof setTimeout> | null = null;

  return create<MapViewState>()(
    subscribeWithSelector((set, get) => ({
      liveViewState: DEFAULT_VIEW,
      settledViewState: DEFAULT_VIEW,
      settledBounds: DEFAULT_BOUNDS,
      setLive: (v, b) => {
        // liveViewState write does not cause renders because no component
        // subscribes to it via a hook (it is read transiently only).
        set({ liveViewState: v });
        pending = { v, b };
        if (timer == null) {
          timer = setTimeout(() => {
            timer = null;
            get().flushSettle();
          }, throttleMs);
        }
      },
      flushSettle: () => {
        if (!pending) return;
        set({ settledViewState: pending.v, settledBounds: pending.b });
        pending = null;
      },
    })),
  );
}

export const useMapViewStore = createMapViewStore();
