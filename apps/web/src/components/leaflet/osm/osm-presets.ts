/**
 * Curated OSM tag filters consumable by `useOsmFeatures` /
 * `OsmFeatureSubtract` / `OsmFeatureAdd`. Each preset compiles to one or more
 * Overpass query clauses targeting `way`/`relation` features whose geometry
 * can be converted to a Polygon (with optional buffering for line features).
 */

import { tagToFilter, type OsmTagInput } from "./osm-tag-catalog";

type OsmTagFilter =
  | { key: string; value: string }
  | { key: string; values: string[] }
  | { key: string; any: true };

interface OsmPresetDef {
  /** Tag filters; OR'd together inside Overpass. */
  filters: OsmTagFilter[];
  /** When set, line geometries (highway etc.) are buffered to polygons. */
  bufferLinesMeters?: number;
  /** Element types to query (default `["way","relation"]`). */
  elementTypes?: Array<"way" | "relation">;
}

const OSM_PRESETS = {
  // ────────────────────────────────────────────────────────────────────────
  // Curated presets — hand-tuned subsets of well-known OSM keys.
  // ────────────────────────────────────────────────────────────────────────
  water: {
    filters: [
      { key: "natural", values: ["water", "bay", "strait"] },
      { key: "waterway", any: true },
    ],
  },
  buildings: {
    filters: [{ key: "building", any: true }],
  },
  forest: {
    filters: [
      { key: "natural", values: ["wood", "forest"] },
      { key: "landuse", value: "forest" },
    ],
  },
  roads: {
    filters: [
      { key: "highway", values: ["motorway", "trunk", "primary", "secondary"] },
    ],
    bufferLinesMeters: 15,
    elementTypes: ["way"],
  },

  // ────────────────────────────────────────────────────────────────────────
  // Category presets — broad catch-alls matching ANY value of an OSM
  // top-level key. Polygon-emitting categories need no buffer; line-likely
  // categories (highway, waterway, railway, barrier, power) ship a small
  // default radius so the polygon-only set operations have geometry to work
  // with. highway/railway narrow elementTypes to ["way"] to skip relation
  // clutter.
  // ────────────────────────────────────────────────────────────────────────
  natural: { filters: [{ key: "natural", any: true }] },
  building: { filters: [{ key: "building", any: true }] },
  landuse: { filters: [{ key: "landuse", any: true }] },
  amenity: { filters: [{ key: "amenity", any: true }] },
  leisure: { filters: [{ key: "leisure", any: true }] },
  highway: {
    filters: [{ key: "highway", any: true }],
    bufferLinesMeters: 5,
    elementTypes: ["way"],
  },
  waterway: {
    filters: [{ key: "waterway", any: true }],
    bufferLinesMeters: 5,
  },
  railway: {
    filters: [{ key: "railway", any: true }],
    bufferLinesMeters: 5,
    elementTypes: ["way"],
  },
  boundary: { filters: [{ key: "boundary", any: true }] },
  place: { filters: [{ key: "place", any: true }] },
  man_made: { filters: [{ key: "man_made", any: true }] },
  shop: { filters: [{ key: "shop", any: true }] },
  tourism: { filters: [{ key: "tourism", any: true }] },
  barrier: {
    filters: [{ key: "barrier", any: true }],
    bufferLinesMeters: 3,
  },
  historic: { filters: [{ key: "historic", any: true }] },
  power: {
    filters: [{ key: "power", any: true }],
    bufferLinesMeters: 3,
  },
} as const satisfies Record<string, OsmPresetDef>;

type OsmPresetName = keyof typeof OSM_PRESETS;

function renderFilter(filter: OsmTagFilter): string {
  if ("value" in filter) return `["${filter.key}"="${filter.value}"]`;
  if ("values" in filter)
    return `["${filter.key}"~"^(${filter.values.join("|")})$"]`;
  return `["${filter.key}"]`;
}

/**
 * Compile an Overpass query from preset names AND/OR raw tag strings.
 * Filters from both sources are OR'd together inside one query.
 */
function buildOverpassQueryFromSources(
  sources: {
    presets?: ReadonlyArray<OsmPresetName>;
    tags?: ReadonlyArray<OsmTagInput>;
  },
  bbox: GeoJSON.BBox,
): string {
  const [w, s, e, n] = bbox;
  const bboxStr = `${s},${w},${n},${e}`;
  const lines: string[] = [];

  for (const name of sources.presets ?? []) {
    const preset: OsmPresetDef = OSM_PRESETS[name];
    const elements = preset.elementTypes ?? ["way", "relation"];
    for (const filter of preset.filters) {
      const tagExpr = renderFilter(filter);
      for (const el of elements) {
        lines.push(`  ${el}${tagExpr}(${bboxStr});`);
      }
    }
  }

  for (const tag of sources.tags ?? []) {
    const filter = tagToFilter(tag);
    const tagExpr = renderFilter(filter);
    for (const el of ["way", "relation"] as const) {
      lines.push(`  ${el}${tagExpr}(${bboxStr});`);
    }
  }

  return `[out:json][timeout:25];\n(\n${lines.join("\n")}\n);\nout geom;`;
}

export {
  type OsmTagFilter,
  type OsmPresetDef,
  OSM_PRESETS,
  type OsmPresetName,
  buildOverpassQueryFromSources,
};
