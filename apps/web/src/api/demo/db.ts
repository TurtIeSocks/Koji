// Hand-rolled IndexedDB wrapper for the demo world — no new dependency —
// with an in-memory Map-backed fallback for environments where IndexedDB
// isn't available (or, in unit tests, is explicitly stubbed out; see
// seeds/seed.test.ts). Every store is keyed by `id` except `meta`, which is
// keyed by `k` (used today for a single `{ k: "seedVersion", v: number }`
// row).
//
// Row shapes mirror `koji-db`'s SeaORM `Model` field names (snake_case) 1:1
// so a later task's demo endpoints can return these rows straight through
// without translation.

/** Bump to force every demo session to wipe + reseed (e.g. after a fixture
 *  shape change) — see seeds/seed.ts's `ensureSeeded`. */
export const SEED_VERSION = 1;

export type StoreName = "geofences" | "routes" | "projects" | "properties" | "webhooks" | "tileservers" | "meta";

const STORE_NAMES: StoreName[] = [
  "geofences",
  "routes",
  "projects",
  "properties",
  "webhooks",
  "tileservers",
  "meta",
];

/** koji_db::db::geofence::Model, plus the `projects`/`properties` keys the
 *  live `get_one_json_with_related` hydrates in (crates/koji-db/src/db/geofence/reads.rs). */
export interface GeofenceRow {
  id: number;
  name: string;
  mode: string;
  parent: number | null;
  geo_type: string;
  geometry: GeoJSON.Polygon;
  min_lat: number | null;
  min_lng: number | null;
  max_lat: number | null;
  max_lng: number | null;
  projects: number[];
  properties: unknown[];
  created_at: string;
  updated_at: string;
}

/** koji_db::db::route::Model. Not seeded until Task 8 (needs the wasm calc
 *  engine); the store exists so Task 8 doesn't need a schema migration. */
export interface RouteRow {
  id: number;
  geofence_id: number;
  name: string;
  description: string | null;
  mode: string;
  geometry: GeoJSON.Geometry;
  points: number;
  created_at: string;
  updated_at: string;
}

/** koji_db::db::project::Model. */
export interface ProjectRow {
  id: number;
  name: string;
  description: string | null;
  created_at: string;
  updated_at: string;
}

/** koji_db::db::property::Model. */
export interface PropertyRow {
  id: number;
  name: string;
  category: string;
  default_value: string | null;
  created_at: string;
  updated_at: string;
}

/** koji_db::db::webhook::Model (`webhook_subscription` table). */
export interface WebhookRow {
  id: number;
  name: string;
  url: string;
  secret: string | null;
  topics: string[];
  active: boolean;
  project_id: number | null;
  mode: string;
  method: string;
  headers: Record<string, string> | null;
  created_at: string;
  updated_at: string;
}

/** koji_db::db::tile_server::Model. */
export interface TileServerRow {
  id: number;
  name: string;
  url: string;
  created_at: string;
  updated_at: string;
}

/** The lone `meta` row this module writes today: `{ k: "seedVersion", v }`. */
export interface MetaRow {
  k: string;
  v: unknown;
}

// A hand-written distributive conditional (not a lookup table) so
// `allRows("geofences")` etc. resolve to the exact row shape above without
// callers ever passing an explicit generic argument.
type RowFor<S extends StoreName> = S extends "geofences"
  ? GeofenceRow
  : S extends "routes"
    ? RouteRow
    : S extends "projects"
      ? ProjectRow
      : S extends "properties"
        ? PropertyRow
        : S extends "webhooks"
          ? WebhookRow
          : S extends "tileservers"
            ? TileServerRow
            : MetaRow;

function keyPathFor(store: StoreName): "id" | "k" {
  return store === "meta" ? "k" : "id";
}

function keyOf(store: StoreName, row: Record<string, unknown>): IDBValidKey {
  return row[keyPathFor(store)] as IDBValidKey;
}

const DB_NAME = "koji-demo-world";
const DB_VERSION = 1;

let dbPromise: Promise<IDBDatabase> | null = null;

/** Opens (and lazily creates/upgrades) the demo IndexedDB database. Only
 *  meaningful when IndexedDB is actually available — callers going through
 *  `allRows`/`putRow`/`deleteRow`/`nextId` never reach this on the
 *  in-memory fallback (see `usingIdb` below). */
export function openDemoDb(): Promise<IDBDatabase> {
  if (dbPromise) return dbPromise;
  dbPromise = new Promise<IDBDatabase>((resolve, reject) => {
    if (typeof indexedDB === "undefined") {
      reject(new Error("openDemoDb: IndexedDB is unavailable in this environment"));
      return;
    }
    const req = indexedDB.open(DB_NAME, DB_VERSION);
    req.onupgradeneeded = () => {
      const db = req.result;
      for (const store of STORE_NAMES) {
        if (!db.objectStoreNames.contains(store)) {
          db.createObjectStore(store, { keyPath: keyPathFor(store) });
        }
      }
    };
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => reject(req.error ?? new Error("openDemoDb: failed to open"));
  });
  return dbPromise;
}

function requestToPromise<T>(req: IDBRequest<T>): Promise<T> {
  return new Promise((resolve, reject) => {
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => reject(req.error ?? new Error("IndexedDB request failed"));
  });
}

/** Runs `fn` against `store`'s real `IDBObjectStore` inside a transaction,
 *  resolving once the transaction commits. Exported for future demo
 *  endpoints (Task 7+) that need a custom query beyond the four CRUD
 *  helpers below; those helpers are themselves built on this. Only used on
 *  the real-IndexedDB path — the in-memory fallback never calls it. */
export async function tx<T>(
  store: StoreName,
  mode: IDBTransactionMode,
  fn: (objectStore: IDBObjectStore) => IDBRequest<T>,
): Promise<T> {
  const db = await openDemoDb();
  const transaction = db.transaction(store, mode);
  const result = await requestToPromise(fn(transaction.objectStore(store)));
  return new Promise<T>((resolve, reject) => {
    transaction.oncomplete = () => resolve(result);
    transaction.onerror = () => reject(transaction.error ?? new Error("IndexedDB transaction failed"));
    transaction.onabort = () => reject(transaction.error ?? new Error("IndexedDB transaction aborted"));
  });
}

// ── In-memory fallback ──────────────────────────────────────────────────
// One Map per store, keyed the same way as the real object stores (`id`, or
// `k` for `meta`). Lives for the page session only — never persisted.
const memory: Record<StoreName, Map<IDBValidKey, Record<string, unknown>>> = Object.fromEntries(
  STORE_NAMES.map((store) => [store, new Map()]),
) as Record<StoreName, Map<IDBValidKey, Record<string, unknown>>>;

/** Whether this session is backed by real IndexedDB (`true`) or the
 *  in-memory fallback (`false`) — a toast surfaces `false` to the user,
 *  since their demo data won't survive a reload. Decided lazily, on first
 *  actual read/write, so tests can force the fallback by stubbing out
 *  `indexedDB` before calling anything (see seeds/seed.test.ts) rather than
 *  depending on whichever way jsdom happens to behave. */
export let persistent = true;
let backendDecided = false;

function usingIdb(): boolean {
  if (!backendDecided) {
    persistent = typeof indexedDB !== "undefined";
    backendDecided = true;
  }
  return persistent;
}

export async function allRows<S extends StoreName>(store: S): Promise<RowFor<S>[]> {
  if (!usingIdb()) return [...memory[store].values()] as unknown as RowFor<S>[];
  return tx(store, "readonly", (os) => os.getAll()) as Promise<RowFor<S>[]>;
}

export async function putRow<S extends StoreName>(store: S, row: RowFor<S>): Promise<void> {
  if (!usingIdb()) {
    const record = row as unknown as Record<string, unknown>;
    memory[store].set(keyOf(store, record), record);
    return;
  }
  await tx(store, "readwrite", (os) => os.put(row));
}

export async function deleteRow(store: StoreName, id: IDBValidKey): Promise<void> {
  if (!usingIdb()) {
    memory[store].delete(id);
    return;
  }
  await tx(store, "readwrite", (os) => os.delete(id));
}

/** Next free integer `id` for a numeric-keyed store (every store except
 *  `meta`, which uses a fixed string key). */
export async function nextId(store: Exclude<StoreName, "meta">): Promise<number> {
  const rows = await allRows(store);
  return rows.reduce((max, row) => Math.max(max, row.id), 0) + 1;
}
