import { useCallback, useState } from "react";

// Shared "last seen after" filter for the golbat marker previews on /map, the
// geofence form, and the route form. The picked local datetime persists here so
// it sticks across those surfaces (v1 kept it in the global map persist store).
const KEY = "koji.map.lastSeen";

/** Format a Date as a `datetime-local` input string (LOCAL time, minute
 *  precision). `toISOString` would emit UTC and shift the displayed clock. */
export function toLocalInput(d: Date): string {
	const p = (n: number) => String(n).padStart(2, "0");
	return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}T${p(d.getHours())}:${p(d.getMinutes())}`;
}

/** Epoch **seconds** for a `datetime-local` string, or `0` when empty/unparseable.
 *  The backend filters `updated > last_seen` against absolute epoch seconds, and
 *  treats `0` as "no filter" — so a cleared picker shows every point. */
export function toEpochSeconds(value: string): number {
	if (!value) return 0;
	const ms = new Date(value).getTime();
	return Number.isNaN(ms) ? 0 : Math.floor(ms / 1000);
}

/** Default: one month ago, minute/second-zeroed — mirrors the v1 map default. */
function defaultValue(): string {
	const d = new Date();
	d.setMonth(d.getMonth() - 1);
	d.setMinutes(0, 0, 0);
	return toLocalInput(d);
}

function load(): string {
	try {
		// A stored "" (user cleared the filter → show all) is respected; only a
		// missing key falls back to the one-month-ago default.
		return localStorage.getItem(KEY) ?? defaultValue();
	} catch {
		return defaultValue();
	}
}

/** The persisted "last seen after" filter. `epoch` (seconds) feeds `useMarkers`;
 *  `value`/`setValue` drive the <LastSeenPicker> input. */
export function useLastSeen() {
	const [value, setValueState] = useState<string>(load);
	const setValue = useCallback((v: string) => {
		setValueState(v);
		try {
			localStorage.setItem(KEY, v);
		} catch {
			// storage unavailable → keep the in-memory value only
		}
	}, []);
	return { value, setValue, epoch: toEpochSeconds(value) };
}
