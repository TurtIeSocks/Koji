/** Calc golbat category → route mode (v1: category auto-sets the route mode).
 *  Values MUST match ROUTE_MODES ids in lib/constants.ts / the Rust Mode enum. */
const MAP: Record<string, string> = {
	spawnpoint: "pokemon",
	pokestop: "quest",
	gym: "fort",
	fort: "fort",
};
export function categoryToRouteMode(category: string): string {
	return MAP[category] ?? "unset";
}
