// Persist the `/map` playground camera across reloads (localStorage). ONLY the
// scratch playground uses this — the embedded workbench maps stay transient and
// fit to their record's geometry, so they must not read/write this key.
const KEY = "koji.map.playground.camera";

export interface StoredCamera {
  longitude: number;
  latitude: number;
  zoom: number;
}

export function loadCamera(): StoredCamera | null {
  try {
    const raw = localStorage.getItem(KEY);
    if (!raw) return null;
    const c = JSON.parse(raw) as Partial<StoredCamera>;
    if (
      typeof c?.longitude === "number" &&
      typeof c?.latitude === "number" &&
      typeof c?.zoom === "number"
    ) {
      return { longitude: c.longitude, latitude: c.latitude, zoom: c.zoom };
    }
  } catch {
    // malformed JSON or storage unavailable → fall back to server start center
  }
  return null;
}

let timer: ReturnType<typeof setTimeout> | null = null;
let pending: StoredCamera | null = null;

/** Debounced write — the camera fires a change per animation frame during a
 *  pan/zoom, so coalesce to at most one write per ~300ms (trailing). */
export function saveCamera(c: StoredCamera): void {
  pending = c;
  if (timer) return;
  timer = setTimeout(() => {
    timer = null;
    const c = pending;
    pending = null;
    if (!c) return;
    try {
      localStorage.setItem(KEY, JSON.stringify(c));
    } catch {
      // quota exceeded / storage unavailable → drop the write
    }
  }, 300);
}
