/** Route mode ↔ golbat data category.
 *  Route modes are the collapsed set {unset, pokemon, fort, quest} (ROUTE_MODES
 *  in lib/constants.ts / the Rust Mode enum). The calc reads the golbat category
 *  from the route's `mode` — so the panel no longer asks for a category. */

const MODE_TO_CATEGORY: Record<string, string> = {
	pokemon: "spawnpoint",
	quest: "pokestop",
	fort: "fort",
	unset: "pokestop",
};

/** route.mode → the golbat category the calc queries. Defaults to pokestop. */
export function routeModeToCategory(mode: string | undefined): string {
	return MODE_TO_CATEGORY[mode ?? ""] ?? "pokestop";
}

const CATEGORY_TO_MODE: Record<string, string> = {
	spawnpoint: "pokemon",
	pokestop: "quest",
	gym: "fort",
	fort: "fort",
};

/** golbat category → route mode (kept for any category-first callers). */
export function categoryToRouteMode(category: string): string {
	return CATEGORY_TO_MODE[category] ?? "unset";
}
