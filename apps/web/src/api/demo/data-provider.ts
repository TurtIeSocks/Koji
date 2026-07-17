/* eslint-disable @typescript-eslint/no-explicit-any */
// The demo CRUD DataProvider — an idb-backed (Map-fallback) stand-in for the
// live `/internal/{resource}` endpoints. Mirrors the live provider's wire
// contract: the same list filters, row shapes, geofence write serialization,
// and getManyReference target→filter translation, but every read/write hits
// the local demo world instead of the network.

import type { DataProvider } from "ra-core";
import { serializeGeofenceWrite } from "../shared/serialize-geofence-write";
import type { GeofenceRow, ProjectRow, StoreName } from "./db";
import { allRows, deleteRow, nextId, putRow } from "./db";
import { buildGeofenceRow, buildRouteRow, geoTypeOf, nowIso } from "./rows";

// Re-exported for src/data-provider.ts, which imports it straight from "@api"
// (same historical path the live provider preserves).
export { serializeGeofenceWrite };

interface ResourceDef {
  store: StoreName | null;
  geo: boolean;
  /** Needs the geofence set to compute a derived `geofences` membership. */
  needsGeofences?: boolean;
}

// null store = handled specially (plugins: always an empty list — the demo
// ships no plugins, matching an empty `route/clustering/bootstrap_plugins`).
const RESOURCE_MAP: Record<string, ResourceDef> = {
  geofence: { store: "geofences", geo: true },
  route: { store: "routes", geo: true },
  project: { store: "projects", geo: false, needsGeofences: true },
  property: { store: "properties", geo: false, needsGeofences: true },
  webhook: { store: "webhooks", geo: false },
  tileserver: { store: "tileservers", geo: false },
  plugins: { store: null, geo: false },
};

// ReferenceManyField target -> list filter param. Explicit because strip-`_id`
// doesn't hold for routes (backend wants `geofenceid`, not `geofence`);
// project_id -> project is the webhook precedent. Mirrors live/data-provider.ts.
const TARGET_TO_PARAM: Record<string, string> = {
  project_id: "project",
  geofence_id: "geofenceid",
};

interface Ctx {
  geofences: GeofenceRow[];
}

// ── filters ─────────────────────────────────────────────────────────────

/** A filter value is "present" (should constrain) only when non-null/non-empty
 *  — ra sends `q: ""` / cleared filters that must be treated as absent. */
function present(v: unknown): boolean {
  return v != null && v !== "";
}

function qMatches(name: string, q: unknown): boolean {
  if (!present(q)) return true;
  return name.toLowerCase().includes(String(q).toLowerCase());
}

/** Filter predicate over the already-projected list row. */
function matchesList(resource: string, row: any, filter: Record<string, unknown>): boolean {
  if (!qMatches(String(row.name ?? ""), filter.q)) return false;
  switch (resource) {
    case "geofence":
      return (
        (!present(filter.mode) || row.mode === filter.mode) &&
        (!present(filter.parent) || row.parent === Number(filter.parent)) &&
        (!present(filter.geotype) || row.geo_type === filter.geotype) &&
        (!present(filter.project) || (row.projects as number[]).includes(Number(filter.project)))
      );
    case "route":
      return (
        (!present(filter.mode) || row.mode === filter.mode) &&
        (!present(filter.geofenceid) || row.geofence_id === Number(filter.geofenceid)) &&
        (!present(filter.pointsmin) || row.points >= Number(filter.pointsmin)) &&
        (!present(filter.pointsmax) || row.points <= Number(filter.pointsmax))
      );
    case "property":
      return !present(filter.category) || row.category === filter.category;
    case "webhook":
      return !present(filter.project) || row.project_id === Number(filter.project);
    default:
      return true;
  }
}

// ── projections ─────────────────────────────────────────────────────────

const geofencesForProject = (projectId: number, ctx: Ctx): number[] =>
  ctx.geofences.filter((g) => g.projects.includes(projectId)).map((g) => g.id);

const geofencesForProperty = (propertyId: number, ctx: Ctx): number[] =>
  ctx.geofences
    .filter((g) => (g.properties as { property_id?: number }[]).some((p) => p?.property_id === propertyId))
    .map((g) => g.id);

/** The paginated-list row shape (mirrors the bespoke `/internal/{resource}`
 *  row serializers). */
function toListRow(resource: string, row: any, ctx: Ctx): any {
  switch (resource) {
    case "geofence":
      return {
        id: row.id,
        name: row.name,
        mode: row.mode,
        parent: row.parent,
        geo_type: row.geo_type,
        projects: row.projects,
        property_count: Array.isArray(row.properties) ? row.properties.length : 0,
      };
    case "route":
      return {
        id: row.id,
        name: row.name,
        description: row.description,
        mode: row.mode,
        geofence_id: row.geofence_id,
        points: row.points,
      };
    case "project":
      return { ...row, geofences: geofencesForProject(row.id, ctx) };
    case "property":
      return { ...row, geofences: geofencesForProperty(row.id, ctx) };
    default:
      return row;
  }
}

/** The single-record shape (getOne/getMany). Geo resources return the
 *  already-flattened record (geometry + geo_type inline) — the ApiSurface
 *  contract is ra-core DataProvider, whose getOne returns records. */
function toRecord(resource: string, row: any, ctx: Ctx): any {
  switch (resource) {
    case "geofence":
    case "route":
      return { ...row, geo_type: geoTypeOf(row.geometry) };
    case "project":
      return { ...row, geofences: geofencesForProject(row.id, ctx) };
    case "property":
      return { ...row, geofences: geofencesForProperty(row.id, ctx) };
    default:
      return row;
  }
}

// ── sorting ─────────────────────────────────────────────────────────────

/** Array values sort by length (a `geofences`/`projects` count column). */
const sortKey = (v: unknown): unknown => (Array.isArray(v) ? v.length : v);

/** Sort projected rows: numeric fields numeric, everything else localeCompare;
 *  an unknown `sortBy` field (absent on every row) falls back to `id`. */
function sortRows(rows: any[], field: string, order: string): any[] {
  const key = rows.some((r) => r[field] !== undefined) ? field : "id";
  const dir = order === "DESC" ? -1 : 1;
  return [...rows].sort((a, b) => {
    const av = sortKey(a[key]);
    const bv = sortKey(b[key]);
    if (typeof av === "number" && typeof bv === "number") return dir * (av - bv);
    return dir * String(av ?? "").localeCompare(String(bv ?? ""));
  });
}

// ── shared read/write helpers ─────────────────────────────────────────────

function defOf(resource: string): ResourceDef {
  const def = RESOURCE_MAP[resource];
  if (!def) throw new Error(`demo dataProvider: unknown resource "${resource}"`);
  return def;
}

async function ctxFor(def: ResourceDef): Promise<Ctx> {
  return def.needsGeofences ? { geofences: await allRows("geofences") } : { geofences: [] };
}

async function recordFor(resource: string, row: any): Promise<any> {
  return toRecord(resource, row, await ctxFor(defOf(resource)));
}

async function listImpl(resource: string, params: any): Promise<{ data: any[]; total: number }> {
  const def = defOf(resource);
  if (def.store === null) return { data: [], total: 0 }; // plugins
  const ctx = await ctxFor(def);
  const raw = await allRows(def.store);
  const rows = raw.map((r) => toListRow(resource, r, ctx));
  const filtered = rows.filter((r) => matchesList(resource, r, params.filter ?? {}));
  const sorted = sortRows(filtered, params.sort.field, params.sort.order);
  const { page, perPage } = params.pagination;
  const start = (page - 1) * perPage;
  return { data: sorted.slice(start, start + perPage), total: filtered.length };
}

/** Sync geofence.projects[] membership (the source of truth) to a project's
 *  edited `geofences` list — the demo equivalent of the backend's
 *  `upsert_related_geofences`. A missing `geofences` field leaves membership
 *  untouched (a metadata-only project patch). */
async function reconcileProjectGeofences(projectId: number, wanted: unknown): Promise<void> {
  if (!Array.isArray(wanted)) return;
  const want = new Set(wanted.map(Number));
  const geofences = await allRows("geofences");
  for (const g of geofences) {
    const has = g.projects.includes(projectId);
    const should = want.has(g.id);
    if (has === should) continue;
    const projects = should ? [...g.projects, projectId] : g.projects.filter((p) => p !== projectId);
    await putRow("geofences", { ...g, projects, updated_at: nowIso() });
  }
}

/** Build the stored row for a create/update on a given resource. */
function buildRow(resource: string, data: any, existing: any, id: number, now: string): any {
  if (resource === "geofence") {
    return buildGeofenceRow(serializeGeofenceWrite(data), existing, id, now);
  }
  if (resource === "route") {
    return buildRouteRow(data, existing, id, now);
  }
  // project/property/webhook/tileserver — plain merge. `geofences` is derived
  // (reconciled separately), so it never persists on the project row.
  const { geofences: _drop, ...rest } = data as ProjectRow & { geofences?: unknown };
  const merged = { ...(existing ?? {}), ...rest };
  return { ...merged, id, created_at: existing?.created_at ?? now, updated_at: now };
}

async function findRow(store: StoreName, id: string | number): Promise<any | null> {
  const rows = await allRows(store);
  return rows.find((r: any) => String(r.id) === String(id)) ?? null;
}

async function createImpl(resource: string, params: any): Promise<{ data: any }> {
  const def = defOf(resource);
  if (def.store === null) throw new Error(`demo dataProvider: ${resource} is read-only`);
  const now = nowIso();
  const id = await nextId(def.store as Exclude<StoreName, "meta">);
  const row = buildRow(resource, params.data, null, id, now);
  await putRow(def.store, row);
  if (resource === "project") await reconcileProjectGeofences(id, params.data.geofences);
  return { data: await recordFor(resource, row) };
}

async function updateImpl(resource: string, params: any): Promise<{ data: any }> {
  const def = defOf(resource);
  if (def.store === null) throw new Error(`demo dataProvider: ${resource} is read-only`);
  const existing = await findRow(def.store, params.id);
  const id = existing?.id ?? Number(params.id);
  const row = buildRow(resource, params.data, existing, id, nowIso());
  await putRow(def.store, row);
  if (resource === "project") await reconcileProjectGeofences(id, params.data.geofences);
  return { data: await recordFor(resource, row) };
}

async function deleteImpl(resource: string, id: string | number): Promise<void> {
  const def = defOf(resource);
  if (def.store === null) throw new Error(`demo dataProvider: ${resource} is read-only`);
  const existing = await findRow(def.store, id);
  await deleteRow(def.store, existing?.id ?? Number(id));
}

// ── provider ──────────────────────────────────────────────────────────────

export const demoBaseDataProvider: DataProvider = {
  getList: (resource, params) => listImpl(resource, params) as any,

  getManyReference: (resource, params) => {
    const filterParam = params.target
      ? (TARGET_TO_PARAM[params.target] ?? params.target.replace(/_id$/, ""))
      : undefined;
    const merged = {
      ...params,
      filter: {
        ...(params.filter ?? {}),
        ...(filterParam ? { [filterParam]: params.id } : {}),
      },
    };
    return listImpl(resource, merged) as any;
  },

  getOne: async (resource, params) => {
    const def = defOf(resource);
    if (def.store === null) throw new Error(`demo dataProvider: ${resource} has no records`);
    const row = await findRow(def.store, params.id);
    if (!row) throw new Error(`demo dataProvider: ${resource} ${params.id} not found`);
    return { data: await recordFor(resource, row) } as any;
  },

  getMany: async (resource, params) => {
    const def = defOf(resource);
    if (def.store === null) return { data: [] } as any;
    const ctx = await ctxFor(def);
    const want = new Set(params.ids.map((id: any) => String(id)));
    const rows = await allRows(def.store);
    return {
      data: rows.filter((r: any) => want.has(String(r.id))).map((r) => toRecord(resource, r, ctx)),
    } as any;
  },

  create: (resource, params) => createImpl(resource, params) as any,
  update: (resource, params) => updateImpl(resource, params) as any,

  delete: async (resource, params) => {
    await deleteImpl(resource, params.id);
    return { data: { id: params.id } } as any;
  },

  updateMany: async (resource, params) => {
    for (const id of params.ids) {
      await updateImpl(resource, { id, data: params.data, previousData: {} });
    }
    return { data: params.ids };
  },

  deleteMany: async (resource, params) => {
    for (const id of params.ids) await deleteImpl(resource, id);
    return { data: params.ids };
  },
};
