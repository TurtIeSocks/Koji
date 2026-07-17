// Builds (and re-builds) the demo world's fixture rows from
// nyc-areas.geo.json — 32 Manhattan neighborhood geofences, 3 projects, 4
// properties, 1 (inactive) webhook, and 2 tile servers. Route seeding needs
// the wasm calc engine and lands in Task 8.
//
// `ensureSeeded()` is the idempotent entry point every demo boot calls: it
// checks `meta.seedVersion` and only wipes+reseeds on a mismatch (a fresh
// db, or a bumped SEED_VERSION after a fixture-shape change).
// `resetDemoWorld()` is the unconditional "start over" version a settings
// page or debug action calls directly.

import { DEFAULT_TILE_URL } from "@/lib/constants";
import type { GeofenceRow, ProjectRow, PropertyRow, RouteRow, TileServerRow, WebhookRow } from "../db";
import { allRows, deleteRow, putRow, SEED_VERSION } from "../db";
import { featureBbox } from "./geometry";
import { DEMO_EPOCH_MS } from "./markers";
import nycAreas from "./nyc-areas.geo.json";

const FIXED_ISO = new Date(DEMO_EPOCH_MS).toISOString();

const GEOFENCE_MODES = ["pokemon", "fort", "quest"] as const;

const DATA_STORES = ["geofences", "routes", "projects", "properties", "webhooks", "tileservers"] as const;

/** Area-weighted polygon centroid (shoelace formula) of a single closed
 *  ring: `[lon, lat]` positions in, `{ lat, lon }` out. All 32 seed
 *  features are simple single-ring Polygons (no holes), so the outer ring
 *  alone is the whole shape. */
function ringCentroid(ring: GeoJSON.Position[]): { lat: number; lon: number } {
  let area = 0;
  let cx = 0;
  let cy = 0;
  for (let i = 0; i < ring.length - 1; i++) {
    const [x0, y0] = ring[i];
    const [x1, y1] = ring[i + 1];
    const cross = x0 * y1 - x1 * y0;
    area += cross;
    cx += (x0 + x1) * cross;
    cy += (y0 + y1) * cross;
  }
  area /= 2;
  if (area === 0) {
    // Degenerate ring — fall back to a plain vertex average rather than
    // dividing by zero (not expected for real neighborhood outlines).
    let sumX = 0;
    let sumY = 0;
    for (const [x, y] of ring) {
      sumX += x;
      sumY += y;
    }
    return { lon: sumX / ring.length, lat: sumY / ring.length };
  }
  return { lon: cx / (6 * area), lat: cy / (6 * area) };
}

/** Downtown/Midtown/Uptown by centroid latitude — matches this seed's real
 *  Manhattan geography (see nyc-areas.geo.json). */
function projectIdForLat(lat: number): number {
  if (lat < 40.73) return 1; // Downtown
  if (lat < 40.78) return 2; // Midtown
  return 3; // Uptown
}

function buildGeofenceRows(): GeofenceRow[] {
  // `nycAreas` is typed straight off the JSON literal's own content (tsc's
  // "bundler" module resolution parses .json imports without
  // `resolveJsonModule` even being set), so a single `as` doesn't
  // sufficiently overlap with GeoJSON.FeatureCollection; go through
  // `unknown` (see the same note in markers.test.ts).
  const fc = nycAreas as unknown as GeoJSON.FeatureCollection;
  return fc.features.map((feature, index): GeofenceRow => {
    const geometry = feature.geometry as GeoJSON.Polygon;
    const bbox = featureBbox(feature);
    const centroid = ringCentroid(geometry.coordinates[0]);
    return {
      id: index + 1,
      name: String(feature.properties?.name ?? `Area ${index + 1}`),
      // The seed fixture ships `mode: "auto_quest"` for every feature (a v1
      // leftover) — remapped deterministically across v2's 3 live modes.
      mode: GEOFENCE_MODES[index % GEOFENCE_MODES.length],
      parent: null,
      geo_type: "Polygon",
      geometry,
      min_lat: bbox.minLat,
      min_lng: bbox.minLon,
      max_lat: bbox.maxLat,
      max_lng: bbox.maxLon,
      projects: [projectIdForLat(centroid.lat)],
      properties: [],
      created_at: FIXED_ISO,
      updated_at: FIXED_ISO,
    };
  });
}

function buildProjectRows(): ProjectRow[] {
  return [
    {
      id: 1,
      name: "Downtown",
      description: "Manhattan south of 40.73°N",
      created_at: FIXED_ISO,
      updated_at: FIXED_ISO,
    },
    {
      id: 2,
      name: "Midtown",
      description: "Manhattan 40.73°N-40.78°N",
      created_at: FIXED_ISO,
      updated_at: FIXED_ISO,
    },
    {
      id: 3,
      name: "Uptown",
      description: "Manhattan north of 40.78°N",
      created_at: FIXED_ISO,
      updated_at: FIXED_ISO,
    },
  ];
}

function buildPropertyRows(): PropertyRow[] {
  return [
    {
      id: 1,
      name: "shiny_only",
      category: "boolean",
      default_value: "false",
      created_at: FIXED_ISO,
      updated_at: FIXED_ISO,
    },
    {
      id: 2,
      name: "quest_reward",
      category: "string",
      default_value: "Rare Candy",
      created_at: FIXED_ISO,
      updated_at: FIXED_ISO,
    },
    {
      id: 3,
      name: "min_iv",
      category: "number",
      default_value: "0",
      created_at: FIXED_ISO,
      updated_at: FIXED_ISO,
    },
    {
      id: 4,
      name: "marker_color",
      category: "color",
      default_value: "#22c55e",
      created_at: FIXED_ISO,
      updated_at: FIXED_ISO,
    },
  ];
}

function buildWebhookRows(): WebhookRow[] {
  return [
    {
      id: 1,
      name: "Demo Geofence Hook",
      url: "https://example.com/hook",
      secret: null,
      topics: ["geofence.updated"],
      active: false,
      project_id: null,
      mode: "event",
      method: "POST",
      headers: null,
      created_at: FIXED_ISO,
      updated_at: FIXED_ISO,
    },
  ];
}

function buildTileServerRows(): TileServerRow[] {
  return [
    {
      id: 1,
      name: "Carto Light",
      // Raster XYZ template, NOT the vector style.json — every tileserver
      // consumer feeds `url` through `rasterStyle()` (map-style.ts), which
      // expects an XYZ template and handles `{s}` (expanded to the a-d
      // subdomains) and `{r}` (stripped). Same shape as DEFAULT_TILE_URL.
      url: "https://{s}.basemaps.cartocdn.com/light_all/{z}/{x}/{y}{r}.png",
      created_at: FIXED_ISO,
      updated_at: FIXED_ISO,
    },
    {
      id: 2,
      name: "CartoDB Voyager",
      url: DEFAULT_TILE_URL,
      created_at: FIXED_ISO,
      updated_at: FIXED_ISO,
    },
  ];
}

async function wipeAllData(): Promise<void> {
  for (const store of DATA_STORES) {
    const rows = await allRows(store);
    for (const row of rows) await deleteRow(store, row.id);
  }
}

async function seedAll(): Promise<void> {
  for (const row of buildGeofenceRows()) await putRow("geofences", row);
  for (const row of buildProjectRows()) await putRow("projects", row);
  for (const row of buildPropertyRows()) await putRow("properties", row);
  for (const row of buildWebhookRows()) await putRow("webhooks", row);
  for (const row of buildTileServerRows()) await putRow("tileservers", row);
}

async function currentSeedVersion(): Promise<number | undefined> {
  const meta = await allRows("meta");
  const row = meta.find((m) => m.k === "seedVersion");
  return typeof row?.v === "number" ? row.v : undefined;
}

/** Idempotent: seeds a fresh db, and reseeds on a SEED_VERSION bump, but is
 *  a no-op when the stored version already matches (calling it twice never
 *  duplicates rows). */
export async function ensureSeeded(): Promise<void> {
  if ((await currentSeedVersion()) === SEED_VERSION) return;
  await wipeAllData();
  await seedAll();
  await putRow("meta", { k: "seedVersion", v: SEED_VERSION });
}

/** Unconditional wipe + reseed, regardless of the stored seed version. */
export async function resetDemoWorld(): Promise<void> {
  await wipeAllData();
  await seedAll();
  await putRow("meta", { k: "seedVersion", v: SEED_VERSION });
}

// ── Route seeding (needs the wasm calc engine) ────────────────────────────
// Routes can't be built from static fixtures — they're the output of a real
// clustering+routing calc. So this is a separate step the demo boot runs after
// the base seed AND after the wasm worker is reachable (see main.tsx). The calc
// runner (`submitCalc`) + job fetcher (`getJob`) are injected (the demo facade's)
// so this stays testable with fakes and free of a circular facade↔markers import.

/** A `submitCalc`-shaped calc runner. */
type SeedSubmitCalc = (body: Record<string, unknown>) => Promise<string>;
/** A `getJob`-shaped record fetcher (only the fields route seeding reads). */
type SeedGetJob = (id: string) => Promise<{
  status: string;
  result?: { data?: unknown; stats?: unknown } | null;
  error?: string | null;
}>;

/** Geofences to seed a demo route for (Downtown neighbourhoods 1 & 2). */
const SEED_ROUTE_FENCE_IDS = [1, 2];

/** Collect every `[lon, lat]` position out of a route FeatureCollection — walks
 *  Point / MultiPoint / LineString / Polygon coordinate nestings down to the
 *  innermost pairs. */
function collectPositions(fc: GeoJSON.FeatureCollection | undefined): GeoJSON.Position[] {
  if (!fc?.features) return [];
  const out: GeoJSON.Position[] = [];
  const walk = (coords: unknown): void => {
    if (!Array.isArray(coords)) return;
    if (typeof coords[0] === "number") {
      out.push(coords as GeoJSON.Position);
      return;
    }
    for (const c of coords) walk(c);
  };
  for (const feature of fc.features) {
    const geometry = feature.geometry as { coordinates?: unknown } | null;
    if (geometry?.coordinates) walk(geometry.coordinates);
  }
  return out;
}

/** Poll a job to a terminal state. Seed-time only — the app's `useCalc` resolves
 *  via realtime, but the boot seed has no React subscription, so it polls the
 *  same facade `getJob` the realtime path also updates. */
async function waitForJob(
  getJob: SeedGetJob,
  id: string,
  timeoutMs = 30_000,
): Promise<Awaited<ReturnType<SeedGetJob>>> {
  const start = Date.now();
  for (;;) {
    const rec = await getJob(id);
    if (rec.status === "succeeded" || rec.status === "failed") return rec;
    if (Date.now() - start > timeoutMs) throw new Error(`seedRoutes: job ${id} timed out`);
    await new Promise((resolve) => setTimeout(resolve, 50));
  }
}

/** Run a real `mode: "route"` calc (spawnpoint, radius 70, minPoints 3) on the
 *  first two geofences and store each result as a route row (`"<Fence> Route"`,
 *  mode `quest`, geometry = MultiPoint of the result coordinates). Idempotent
 *  (only when the routes store is empty); skips with a `console.warn` when the
 *  wasm calc is unavailable, so the demo is still usable without seed routes. */
export async function seedRoutes(submitCalc: SeedSubmitCalc, getJob: SeedGetJob): Promise<void> {
  if ((await allRows("routes")).length > 0) return; // idempotent
  const fences = await allRows("geofences");
  let nextRouteId = 1;
  for (const fenceId of SEED_ROUTE_FENCE_IDS) {
    const fence = fences.find((f) => f.id === fenceId);
    if (!fence) continue;
    try {
      const id = await submitCalc({
        mode: "route",
        category: "spawnpoint",
        area: {
          type: "FeatureCollection",
          features: [{ type: "Feature", properties: {}, geometry: fence.geometry }],
        },
        clustering: { calculationMode: "radius", radius: 70, minPoints: 3 },
      });
      const rec = await waitForJob(getJob, id);
      if (rec.status !== "succeeded") {
        console.warn(
          `seedRoutes: calc for "${fence.name}" ${rec.status}${rec.error ? ` (${rec.error})` : ""}; skipping route`,
        );
        continue;
      }
      const positions = collectPositions(rec.result?.data as GeoJSON.FeatureCollection | undefined);
      const geometry: GeoJSON.MultiPoint = { type: "MultiPoint", coordinates: positions };
      const row: RouteRow = {
        id: nextRouteId++,
        geofence_id: fence.id,
        name: `${fence.name} Route`,
        description: null,
        mode: "quest",
        geometry,
        points: positions.length,
        created_at: FIXED_ISO,
        updated_at: FIXED_ISO,
      };
      await putRow("routes", row);
    } catch (err) {
      console.warn(`seedRoutes: wasm calc unavailable for "${fence.name}"; skipping route seed`, err);
    }
  }
}
