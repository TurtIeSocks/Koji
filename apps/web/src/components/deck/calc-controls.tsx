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
import { getAlgorithms } from "@api";
import type { CalcMode, CalcStrategy } from "@/map/lib/calc-request";
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

export interface CalcControlsProps {
	calc: UseCalcReturn;
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
	onRun,
	disabled,
	disabledReason,
	header,
	footer,
}: CalcControlsProps) {
	const { params, setParams, job, error, clear } = calc;
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
					<Button
						type="button"
						size="sm"
						variant="ghost"
						onClick={clear}
						title="Dismiss this calc result and its map overlay, returning to the Calculate button. A job already running on the server is not cancelled."
					>
						Clear result
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
