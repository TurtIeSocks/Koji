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

export const PROPERTY_CATEGORIES = [
  { id: "boolean", name: "Boolean" },
  { id: "string", name: "String" },
  { id: "number", name: "Number" },
  { id: "object", name: "Object (JSON)" },
  { id: "array", name: "Array (JSON)" },
  { id: "database", name: "Database (runtime)" },
  { id: "color", name: "Color" },
] as const;

// Route modes mirror geofence modes — route.mode uses the same DB enum
// (koji_core::Mode: Unset/Pokemon/Fort/Quest, confirmed in sea_orm_active_enums.rs).
export const ROUTE_MODES = GEOFENCE_MODES;

export const WEBHOOK_MODES = [
  { id: "event", name: "Event (signed POST)" },
  { id: "ping", name: "Ping (legacy reload)" },
] as const;

export const WEBHOOK_METHODS = [
  { id: "GET", name: "GET" },
  { id: "POST", name: "POST" },
] as const;

export const WEBHOOK_TOPICS = [
  { id: "project.updated", name: "project.updated" },
  { id: "project.deleted", name: "project.deleted" },
  { id: "project.geofences_changed", name: "project.geofences_changed" },
  { id: "geofence.updated", name: "geofence.updated" },
  { id: "route.updated", name: "route.updated" },
] as const;
