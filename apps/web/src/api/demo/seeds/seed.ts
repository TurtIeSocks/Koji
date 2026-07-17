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
import type { GeofenceRow, ProjectRow, PropertyRow, TileServerRow, WebhookRow } from "../db";
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
      url: "https://basemaps.cartocdn.com/gl/positron-gl-style/style.json",
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
