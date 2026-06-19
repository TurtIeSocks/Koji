export const INTERNAL_BASE = "/internal";

// CartoDB Voyager — Koji's default tile server today. A later spec reads
// /internal tile-servers; hardcoded for the foundation.
export const DEFAULT_TILE_URL =
  "https://{s}.basemaps.cartocdn.com/rastertiles/voyager/{z}/{x}/{y}{r}.png";

/**
 * Canonical geofence mode set. v1's 12 RDM/Unown modes
 * (auto_pokemon/auto_quest/auto_tth/pokemon_iv/circle_*) collapse to v2's 4.
 * SelectInput choices derive from this list.
 */
export const GEOFENCE_MODES = [
  { id: "unset", name: "Unset" },
  { id: "pokemon", name: "Pokémon" },
  { id: "fort", name: "Fort" },
  { id: "quest", name: "Quest" },
] as const;

export const GEOMETRY_TYPES = [
  { id: "Polygon", name: "Polygon" },
  { id: "MultiPolygon", name: "MultiPolygon" },
] as const;
