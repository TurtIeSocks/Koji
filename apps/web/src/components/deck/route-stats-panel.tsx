import { Minus, Plus, X } from "lucide-react";
import { useEffect, useState } from "react";
import {
	formatDuration,
	formatInt,
	formatMeters,
	formatPercent,
} from "@/map/lib/format";

/** Labeled grid of a job's `algorithms::stats::Stats` blob — only rows whose
 *  value is a finite number render. Keys are the snake_case wire names from
 *  crates/algorithms/src/stats.rs. Every number goes through Intl. */
function StatsGrid({ stats }: { stats: unknown }) {
	const s = (stats ?? {}) as Record<string, unknown>;
	const sc = (s.score_components ?? {}) as Record<string, unknown>;
	const n = (v: unknown): number | undefined =>
		typeof v === "number" && Number.isFinite(v) ? v : undefined;

	const covered = n(s.points_covered);
	const totalPoints = n(s.total_points);
	const coverage =
		covered != null && totalPoints
			? `${formatInt(covered)} / ${formatInt(totalPoints)} (${formatPercent(covered / totalPoints)})`
			: covered != null
				? formatInt(covered)
				: undefined;
	const best = n(s.best_cluster_point_count);
	const worst = n(s.worst_cluster_point_count);
	const totalClusters = n(s.total_clusters);
	const distance = n(s.total_distance);
	const longest = n(s.longest_distance);
	const score = n(s.mygod_score);
	const quality = n(sc.quality);
	// route_est_m is a DISTANCE (meters); route_est_s is the cooldown-time estimate
	// for the same tour. The old panel showed only the seconds — hence the stray `s`.
	const routeEstM = n(sc.route_est_m);
	const routeEstS = n(sc.route_est_s);
	const knife = n(sc.knife_edge);
	const overlap = n(sc.overlap_excess);
	const clusterTime = n(s.cluster_time);
	const routeTime = n(s.route_time);

	const rows: [string, string | undefined][] = [
		["Clusters", totalClusters != null ? formatInt(totalClusters) : undefined],
		["Coverage", coverage],
		["Distance", distance != null ? formatMeters(distance) : undefined],
		["Longest hop", longest != null ? formatMeters(longest) : undefined],
		[
			"Cluster pts (best/worst)",
			best != null || worst != null
				? `${best != null ? formatInt(best) : "–"} / ${worst != null ? formatInt(worst) : "–"}`
				: undefined,
		],
		["Score", score != null ? formatInt(score) : undefined],
		// quality is 0.0 unless the lower bound was computed (min_points === 1), so
		// treat 0 as "not computed" and hide the row rather than show a bogus 0%.
		["Quality", quality ? formatPercent(quality) : undefined],
		["Route est.", routeEstM != null ? formatMeters(routeEstM) : undefined],
		[
			"Cooldown est.",
			routeEstS != null ? formatDuration(routeEstS) : undefined,
		],
		["Knife-edge", knife != null ? formatInt(knife) : undefined],
		["Overlap", overlap != null ? formatInt(overlap) : undefined],
		[
			"Cluster time",
			clusterTime != null ? formatDuration(clusterTime) : undefined,
		],
		["Route time", routeTime != null ? formatDuration(routeTime) : undefined],
	];
	const shown = rows.filter((r): r is [string, string] => r[1] != null);
	if (shown.length === 0) return null;
	return (
		<dl className="grid grid-cols-2 gap-x-3 gap-y-1 text-xs">
			{shown.map(([label, value]) => (
				<div key={label} className="contents">
					<dt className="text-muted-foreground">{label}</dt>
					<dd className="text-right tabular-nums">{value}</dd>
				</div>
			))}
		</dl>
	);
}

export interface RouteStatsPanelProps {
	stats: unknown;
	/** Show a "Computing…" state while a stats job is in flight (no stats yet). */
	loading?: boolean;
	title?: string;
}

/** Floating stats window pinned to the map's bottom-right, minimizable (collapse
 *  to its title bar) and closable (dismiss). Reopens itself when a new stats blob
 *  arrives (e.g. a fresh calc), so closing it is per-result, not permanent. Must
 *  render inside a positioned (relative) map container. */
export function RouteStatsPanel({
	stats,
	loading,
	title = "Route stats",
}: RouteStatsPanelProps) {
	const [open, setOpen] = useState(true);
	const [minimized, setMinimized] = useState(false);

	// A new stats object → reopen (closing dismisses only the current result).
	useEffect(() => {
		if (stats != null) setOpen(true);
	}, [stats]);

	if (!open) return null;
	// Nothing to show yet and nothing computing → don't clutter the map.
	if (stats == null && !loading) return null;

	return (
		<div className="absolute right-2 bottom-2 z-20 w-64 overflow-hidden rounded-lg border bg-background/95 shadow-lg backdrop-blur">
			<div className="flex items-center justify-between border-b px-3 py-1.5">
				<span className="text-xs font-medium">{title}</span>
				<div className="flex items-center gap-0.5">
					<button
						type="button"
						aria-label={
							minimized ? "Expand route stats" : "Minimize route stats"
						}
						className="rounded p-0.5 text-muted-foreground hover:bg-muted hover:text-foreground"
						onClick={() => setMinimized((m) => !m)}
					>
						{minimized ? (
							<Plus className="size-3.5" />
						) : (
							<Minus className="size-3.5" />
						)}
					</button>
					<button
						type="button"
						aria-label="Close route stats"
						className="rounded p-0.5 text-muted-foreground hover:bg-muted hover:text-foreground"
						onClick={() => setOpen(false)}
					>
						<X className="size-3.5" />
					</button>
				</div>
			</div>
			{!minimized && (
				<div className="p-3">
					{stats == null && loading ? (
						<span className="text-xs text-muted-foreground">Computing…</span>
					) : (
						<StatsGrid stats={stats} />
					)}
				</div>
			)}
		</div>
	);
}
