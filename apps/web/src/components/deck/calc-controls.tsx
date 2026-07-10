import { useQuery } from "@tanstack/react-query";
import type { ReactNode } from "react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import { getAlgorithms } from "@/map/data/calc-client";
import type { CalcMode, CalcStrategy, TthFilter } from "@/map/lib/calc-request";
import type { UseCalcReturn } from "./use-calc";

const MODES: { mode: CalcMode; label: string }[] = [
	{ mode: "cluster", label: "Cluster" },
	{ mode: "bootstrap", label: "Bootstrap" },
];
const STRATEGIES: { strategy: CalcStrategy; label: string }[] = [
	{ strategy: "radius", label: "Radius" },
	{ strategy: "s2", label: "S2" },
];
const S2_LEVELS = [10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20];
const S2_SIZES = [1, 3, 5, 7, 9];
const TTHS: TthFilter[] = ["All", "Known", "Unknown"];

export interface CalcControlsProps {
	calc: UseCalcReturn;
	/** Golbat category derived from the route's mode — shown read-only; drives
	 *  whether the spawnpoint-only Tth control is offered. */
	category: string;
	onRun: () => void;
	disabled?: boolean;
	disabledReason?: string;
	/** Optional content pinned at the TOP of the dock (e.g. the playground's
	 *  "← Admin" back link). */
	header?: ReactNode;
	/** Optional trailing content pinned at the bottom of the dock (e.g. a
	 *  "Save" button in the playground). */
	footer?: ReactNode;
}

/** Presentational calc panel — driven entirely by the `calc` prop (`useCalc`),
 *  no global store. The parent owns the area input, the derived `category`, and
 *  gating; this renders params/progress and calls `onRun`. */
export function CalcControls({
	calc,
	category,
	onRun,
	disabled,
	disabledReason,
	header,
	footer,
}: CalcControlsProps) {
	const { params, setParams, job, stats, error, clear } = calc;
	const {
		mode,
		strategy,
		radius,
		s2Level,
		s2Size,
		minPoints,
		clusterMode,
		maxClusters,
		centerClusters,
		sortBy,
		tth,
	} = params;

	// Algorithm option lists (GET /algorithms). If the endpoint is down the
	// selects fall back to just Default and the server default applies.
	const { data: algorithms } = useQuery({
		queryKey: ["calc", "algorithms"],
		queryFn: getAlgorithms,
		staleTime: Number.POSITIVE_INFINITY,
	});

	const isCluster = mode === "cluster";
	const isRadius = strategy === "radius";
	const isS2 = strategy === "s2";
	const showRadius = isRadius;
	const showS2 = isS2;
	const showClusterKnobs = isCluster && isRadius; // clusterMode / maxClusters / centerClusters
	const showMaxClusters = showClusterKnobs && clusterMode !== "fastest";
	const showTth = isCluster && category === "spawnpoint";
	const blocked = !!disabled;

	return (
		// Left-dock content — fills whatever positioned wrapper the consumer gives
		// it (RouteMap: full height; playground: stops above the draw toolbar).
		// Right border instead of a shadow so it reads as a sidebar of the map, not
		// a floating card. Scrolls independently when the controls overflow.
		<div className="flex h-full w-60 flex-col gap-3 overflow-y-auto border-r bg-background/95 p-3 backdrop-blur">
			{header}
			<div className="flex items-center justify-between">
				<span className="text-sm font-medium">Calculate</span>
				{job && (
					<Badge
						variant={job.status === "failed" ? "destructive" : "secondary"}
					>
						{job.status}
					</Badge>
				)}
			</div>

			<Field label="Mode">
				<Select
					value={mode}
					onValueChange={(v) => setParams({ mode: v as CalcMode })}
				>
					<SelectTrigger aria-label="Mode" className="w-full">
						<SelectValue />
					</SelectTrigger>
					<SelectContent>
						{MODES.map((m) => (
							<SelectItem key={m.mode} value={m.mode}>
								{m.label}
							</SelectItem>
						))}
					</SelectContent>
				</Select>
			</Field>

			{/* <div className="flex flex-col gap-0.5">
				<Label className="text-xs text-muted-foreground">Category (from route mode)</Label>
				<span className="text-sm capitalize">{category}</span>
			</div>
 */}
			{showTth && (
				<Field label="Tth">
					<Select
						value={tth}
						onValueChange={(v) => setParams({ tth: v as TthFilter })}
					>
						<SelectTrigger aria-label="Tth" className="w-full">
							<SelectValue />
						</SelectTrigger>
						<SelectContent>
							{TTHS.map((t) => (
								<SelectItem key={t} value={t}>
									{t}
								</SelectItem>
							))}
						</SelectContent>
					</Select>
				</Field>
			)}

			<Field label="Strategy">
				<Select
					value={strategy}
					onValueChange={(v) => setParams({ strategy: v as CalcStrategy })}
				>
					<SelectTrigger aria-label="Strategy" className="w-full">
						<SelectValue />
					</SelectTrigger>
					<SelectContent>
						{STRATEGIES.map((s) => (
							<SelectItem key={s.strategy} value={s.strategy}>
								{s.label}
							</SelectItem>
						))}
					</SelectContent>
				</Select>
			</Field>

			{showClusterKnobs && (
				<Field label="Clustering mode">
					<Select
						value={clusterMode ?? "default"}
						onValueChange={(v) =>
							setParams({ clusterMode: v === "default" ? null : v })
						}
					>
						<SelectTrigger aria-label="Clustering mode" className="w-full">
							<SelectValue />
						</SelectTrigger>
						<SelectContent>
							<SelectItem value="default">Default</SelectItem>
							{(algorithms?.clustering ?? []).map((c) => (
								<SelectItem key={c} value={c}>
									{c}
								</SelectItem>
							))}
						</SelectContent>
					</Select>
				</Field>
			)}

			{showRadius && (
				<Field label="Radius (m)">
					<Input
						type="number"
						aria-label="Radius"
						value={radius}
						min={1}
						onChange={(e) => setParams({ radius: Number(e.target.value) })}
					/>
				</Field>
			)}

			{showS2 && (
				<>
					<Field label="S2 level">
						<Select
							value={String(s2Level)}
							onValueChange={(v) => setParams({ s2Level: Number(v) })}
						>
							<SelectTrigger aria-label="S2 level" className="w-full">
								<SelectValue />
							</SelectTrigger>
							<SelectContent>
								{S2_LEVELS.map((l) => (
									<SelectItem key={l} value={String(l)}>
										Level {l}
									</SelectItem>
								))}
							</SelectContent>
						</Select>
					</Field>
					<Field label="S2 size">
						<Select
							value={String(s2Size)}
							onValueChange={(v) => setParams({ s2Size: Number(v) })}
						>
							<SelectTrigger aria-label="S2 size" className="w-full">
								<SelectValue />
							</SelectTrigger>
							<SelectContent>
								{S2_SIZES.map((s) => (
									<SelectItem key={s} value={String(s)}>
										{s}x{s}
									</SelectItem>
								))}
							</SelectContent>
						</Select>
					</Field>
				</>
			)}

			{isCluster && (
				<Field label="Min points">
					<Input
						type="number"
						aria-label="Min points"
						value={minPoints}
						min={1}
						onChange={(e) => setParams({ minPoints: Number(e.target.value) })}
					/>
				</Field>
			)}

			{showMaxClusters && (
				<Field label="Max clusters (0 = unlimited)">
					<Input
						type="number"
						aria-label="Max clusters"
						value={maxClusters ?? 0}
						min={0}
						onChange={(e) => setParams({ maxClusters: Number(e.target.value) })}
					/>
				</Field>
			)}

			{showClusterKnobs && (
				<div className="flex items-center justify-between">
					<Label
						htmlFor="center-clusters"
						className="text-xs text-muted-foreground"
					>
						Center clusters
					</Label>
					<Switch
						id="center-clusters"
						aria-label="Center clusters"
						checked={centerClusters}
						onCheckedChange={(v) => setParams({ centerClusters: v })}
					/>
				</div>
			)}

			<Field label="Routing algorithm">
				<Select
					value={sortBy ?? "default"}
					onValueChange={(v) =>
						setParams({ sortBy: v === "default" ? null : v })
					}
				>
					<SelectTrigger aria-label="Routing algorithm" className="w-full">
						<SelectValue />
					</SelectTrigger>
					<SelectContent>
						<SelectItem value="default">Default (TSP)</SelectItem>
						<SelectItem value="unset">None</SelectItem>
						{(algorithms?.routing ?? []).map((r) => (
							<SelectItem key={r} value={r}>
								{r}
							</SelectItem>
						))}
					</SelectContent>
				</Select>
			</Field>

			{job ? (
				<div className="flex flex-col gap-1">
					<div className="h-1.5 w-full overflow-hidden rounded bg-muted">
						<div
							className="h-full bg-primary transition-[width]"
							style={{ width: `${Math.round(job.progress * 100)}%` }}
						/>
					</div>
					{job.phase && (
						<span className="text-xs text-muted-foreground">{job.phase}</span>
					)}
					{error && <span className="text-xs text-destructive">{error}</span>}
					{stats != null && <CalcStats stats={stats} />}
					<Button type="button" size="sm" variant="ghost" onClick={clear}>
						Clear
					</Button>
				</div>
			) : (
				<>
					<Button type="button" size="sm" disabled={blocked} onClick={onRun}>
						Calculate
					</Button>
					{blocked && disabledReason && (
						<span className="text-xs text-muted-foreground">
							{disabledReason}
						</span>
					)}
					{error && !blocked && (
						<span className="text-xs text-destructive">{error}</span>
					)}
				</>
			)}
			{footer && <div className="mt-auto border-t pt-3">{footer}</div>}
		</div>
	);
}

function Field({
	label,
	children,
}: {
	label: string;
	children: React.ReactNode;
}) {
	return (
		<div className="flex flex-col gap-1">
			<Label className="text-xs text-muted-foreground">{label}</Label>
			{children}
		</div>
	);
}

/** Labeled grid of the job's `algorithms::stats::Stats` blob (best-effort — only
 *  rows whose value is a finite number render). Field names are the snake_case
 *  wire keys from crates/algorithms/src/stats.rs. */
function CalcStats({ stats }: { stats: unknown }) {
	const s = (stats ?? {}) as Record<string, unknown>;
	const sc = (s.score_components ?? {}) as Record<string, unknown>;
	const n = (v: unknown): number | undefined =>
		typeof v === "number" && Number.isFinite(v) ? v : undefined;
	const meters = (m: number) =>
		m >= 1000 ? `${(m / 1000).toFixed(2)} km` : `${Math.round(m)} m`;
	const secs = (x: number) =>
		x < 1 ? `${Math.round(x * 1000)} ms` : `${x.toFixed(2)} s`;

	const covered = n(s.points_covered);
	const totalPoints = n(s.total_points);
	const coverage =
		covered != null && totalPoints
			? `${covered} / ${totalPoints} (${Math.round((covered / totalPoints) * 100)}%)`
			: covered != null
				? String(covered)
				: undefined;
	const best = n(s.best_cluster_point_count);
	const worst = n(s.worst_cluster_point_count);
	const totalClusters = n(s.total_clusters);
	const distance = n(s.total_distance);
	const longest = n(s.longest_distance);
	const score = n(s.mygod_score);
	const quality = n(sc.quality);
	const routeEst = n(sc.route_est_s);
	const knife = n(sc.knife_edge);
	const overlap = n(sc.overlap_excess);
	const clusterTime = n(s.cluster_time);
	const routeTime = n(s.route_time);

	const rows: [string, string | undefined][] = [
		["Clusters", totalClusters != null ? String(totalClusters) : undefined],
		["Coverage", coverage],
		["Distance", distance != null ? meters(distance) : undefined],
		["Longest hop", longest != null ? meters(longest) : undefined],
		[
			"Cluster pts (best/worst)",
			best != null || worst != null ? `${best ?? "–"} / ${worst ?? "–"}` : undefined,
		],
		["Score", score != null ? score.toLocaleString() : undefined],
		// quality is 0.0 unless the lower bound was computed (min_points === 1), so
		// treat 0 as "not computed" and hide the row rather than show a bogus 0%.
		["Quality", quality ? `${Math.round(quality * 100)}%` : undefined],
		["Route est.", routeEst != null ? secs(routeEst) : undefined],
		["Knife-edge", knife != null ? String(knife) : undefined],
		["Overlap", overlap != null ? String(overlap) : undefined],
		["Cluster time", clusterTime != null ? secs(clusterTime) : undefined],
		["Route time", routeTime != null ? secs(routeTime) : undefined],
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
