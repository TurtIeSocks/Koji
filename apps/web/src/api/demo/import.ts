// Demo mirror of the atomic `POST /internal/import` bulk import + the
// `POST /internal/geometry/convert` normalisation. `postImport` computes a
// dry-run summary against existing rows, then (on commit) writes them; wasm
// backs `postConvert` only (dynamic-imported, shared singleton — see wasm.ts).

import type {
  GeofenceRow,
  RouteRow,
} from "./db";
import { allRows, putRow } from "./db";
import { buildGeofenceRow, buildRouteRow, nowIso } from "./rows";
import { getWasm } from "./wasm";
import type {
  ImportAction,
  ImportItem,
  ImportOutcome,
  ImportRequest,
  ImportResult,
} from "../types";

/** Normalise a FeatureCollection through `@koji-wasm`'s `convert_geometry`
 *  (the same `KojiGeometryCollection` round-trip the server runs). Returns the
 *  normalised features. */
export async function postConvert(features: unknown[]): Promise<unknown[]> {
  const wasm = await getWasm();
  const out = wasm.convert_geometry({ type: "FeatureCollection", features }) as
    | { features?: unknown[] }
    | unknown[];
  if (Array.isArray(out)) return out;
  return out?.features ?? [];
}

/** The write to apply once an item's action is decided — held in memory so the
 *  commit is a single pass after every item has been classified (no partial
 *  writes from a fail discovered mid-loop). */
interface PlannedWrite {
  store: "geofences" | "routes";
  row: GeofenceRow | RouteRow;
}

interface Classified {
  outcome: ImportOutcome;
  write?: PlannedWrite;
}

/** Classify one geofence item against the current name→row index. */
function classifyGeofence(
  item: ImportItem,
  index: number,
  byName: Map<string, GeofenceRow>,
  now: string,
  assignId: () => number,
): Classified {
  const existing = byName.get(item.name);
  if (existing) {
    if (item.on_collision === "overwrite") {
      const row = buildGeofenceRow(
        { name: item.name, mode: item.mode, geometry: item.geometry, projects: item.projects },
        existing,
        existing.id,
        now,
      );
      return {
        outcome: { index, name: item.name, action: "update", id: existing.id, reason: null },
        write: { store: "geofences", row },
      };
    }
    return {
      outcome: { index, name: item.name, action: "skip", id: existing.id, reason: "name exists" },
    };
  }
  const id = assignId();
  const row = buildGeofenceRow(
    { name: item.name, mode: item.mode, geometry: item.geometry, projects: item.projects },
    null,
    id,
    now,
  );
  return {
    outcome: { index, name: item.name, action: "create", id, reason: null },
    write: { store: "geofences", row },
  };
}

/** Classify one route item — its parent geofence (by name via `route_parent`)
 *  must resolve, else the item fails. */
function classifyRoute(
  item: ImportItem,
  index: number,
  byName: Map<string, RouteRow>,
  geofenceByName: Map<string, GeofenceRow>,
  now: string,
  assignId: () => number,
): Classified {
  const parent = item.route_parent ? geofenceByName.get(item.route_parent) : undefined;
  if (!parent) {
    return {
      outcome: {
        index,
        name: item.name,
        action: "fail",
        id: null,
        reason: `unknown parent geofence "${item.route_parent ?? ""}"`,
      },
    };
  }
  const base = {
    name: item.name,
    mode: item.mode,
    geometry: item.geometry,
    geofence_id: parent.id,
  };
  const existing = byName.get(item.name);
  if (existing) {
    if (item.on_collision === "overwrite") {
      const row = buildRouteRow(base, existing, existing.id, now);
      return {
        outcome: { index, name: item.name, action: "update", id: existing.id, reason: null },
        write: { store: "routes", row },
      };
    }
    return {
      outcome: { index, name: item.name, action: "skip", id: existing.id, reason: "name exists" },
    };
  }
  const id = assignId();
  const row = buildRouteRow(base, null, id, now);
  return {
    outcome: { index, name: item.name, action: "create", id, reason: null },
    write: { store: "routes", row },
  };
}

export async function postImport(body: ImportRequest): Promise<ImportResult> {
  const now = nowIso();
  const geofences = await allRows("geofences");
  const routes = await allRows("routes");
  const geofenceByName = new Map(geofences.map((r) => [r.name, r]));
  const routeByName = new Map(routes.map((r) => [r.name, r]));

  // Monotonic id allocators — seeded from the current max so classification can
  // hand out ids without hitting the db per item.
  let nextGeoId = geofences.reduce((m, r) => Math.max(m, r.id), 0) + 1;
  let nextRouteId = routes.reduce((m, r) => Math.max(m, r.id), 0) + 1;

  const classified = body.items.map((item, index) =>
    item.kind === "route"
      ? classifyRoute(item, index, routeByName, geofenceByName, now, () => nextRouteId++)
      : classifyGeofence(item, index, geofenceByName, now, () => nextGeoId++),
  );

  const summary = { create: 0, update: 0, skip: 0, fail: 0 };
  for (const { outcome } of classified) {
    summary[outcome.action as ImportAction] += 1;
  }
  const results = classified.map((c) => c.outcome);

  if (!body.dry_run) {
    for (const { write } of classified) {
      if (!write) continue;
      if (write.store === "geofences") await putRow("geofences", write.row as GeofenceRow);
      else await putRow("routes", write.row as RouteRow);
    }
  }

  return { committed: !body.dry_run, summary, results };
}
