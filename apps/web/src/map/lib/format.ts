// Locale-aware number formatting (Intl.NumberFormat) — one place so every stat
// reads consistently and respects the viewer's locale (grouping, decimals, units).
// Formatters are constructed once (they're relatively expensive) and reused.

const int = new Intl.NumberFormat(undefined, { maximumFractionDigits: 0 });
/** Grouped integer, e.g. `1,234`. */
export const formatInt = (n: number): string => int.format(n);

const pct = new Intl.NumberFormat(undefined, {
	style: "percent",
	maximumFractionDigits: 0,
});
/** A 0..1 ratio as a percent, e.g. `0.83` → `83%`. */
export const formatPercent = (ratio: number): string => pct.format(ratio);

const meters = new Intl.NumberFormat(undefined, {
	style: "unit",
	unit: "meter",
	maximumFractionDigits: 0,
});
const kilometers = new Intl.NumberFormat(undefined, {
	style: "unit",
	unit: "kilometer",
	maximumFractionDigits: 2,
});
/** A distance in METERS, shown as `m` under 1 km and `km` above. */
export const formatMeters = (m: number): string =>
	m >= 1000 ? kilometers.format(m / 1000) : meters.format(Math.round(m));

const millis = new Intl.NumberFormat(undefined, {
	style: "unit",
	unit: "millisecond",
	maximumFractionDigits: 0,
});
const seconds = new Intl.NumberFormat(undefined, {
	style: "unit",
	unit: "second",
	maximumFractionDigits: 1,
});
const minutes = new Intl.NumberFormat(undefined, {
	style: "unit",
	unit: "minute",
	maximumFractionDigits: 1,
});
/** A duration in SECONDS: `ms` under 1 s, `s` up to 90 s, `min` beyond. */
export const formatDuration = (s: number): string => {
	if (s < 1) return millis.format(Math.round(s * 1000));
	if (s <= 90) return seconds.format(s);
	return minutes.format(s / 60);
};
