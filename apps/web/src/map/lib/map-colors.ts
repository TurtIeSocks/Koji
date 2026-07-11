// Single source of truth for every map layer's color, so markers, geofences,
// routes, cluster centers, and S2 cells each read as a DISTINCT hue (they used
// to be scattered inline across the three map hosts — spawnpoints and routes
// both landed on green). RGB triples (0–255); alpha is applied at each layer.
export type RGB = [number, number, number];

type ColorKey =
	| "gym"
	| "pokestop"
	| "spawnpoint"
	| "station"
	| "geofence"
	| "route"
	| "calcCenter"
	| "s2";

export const COLOR: Record<ColorKey, RGB> = {
	gym: [230, 80, 80], // red
	pokestop: [0, 120, 255], // blue
	spawnpoint: [40, 200, 120], // green
	station: [150, 80, 220], // purple
	geofence: [255, 140, 0], // orange
	route: [0, 200, 210], // cyan — was green, collided with spawnpoints
	calcCenter: [255, 0, 200], // magenta — cluster centers + coverage circles
	s2: [255, 0, 0], // red grid lines (unfilled, no collision with gym dots)
};
