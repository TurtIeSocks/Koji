import { useQuery } from "@tanstack/react-query";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Badge } from "@/components/ui/badge";
import { Switch } from "@/components/ui/switch";
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "@/components/ui/select";
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
}

/** Presentational calc panel — driven entirely by the `calc` prop (`useCalc`),
 *  no global store. The parent owns the area input, the derived `category`, and
 *  gating; this renders params/progress and calls `onRun`. */
export function CalcControls({ calc, category, onRun, disabled, disabledReason }: CalcControlsProps) {
	const { params, setParams, job, stats, error, clear } = calc;
	const { mode, strategy, radius, s2Level, s2Size, minPoints, clusterMode, maxClusters, centerClusters, sortBy, tth } = params;

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
		<div className="flex max-h-[460px] w-64 flex-col gap-3 overflow-y-auto rounded-lg border bg-background/90 p-3 shadow-md backdrop-blur">
			<div className="flex items-center justify-between">
				<span className="text-sm font-medium">Calculate</span>
				{job && <Badge variant={job.status === "failed" ? "destructive" : "secondary"}>{job.status}</Badge>}
			</div>

			<Field label="Mode">
				<Select value={mode} onValueChange={(v) => setParams({ mode: v as CalcMode })}>
					<SelectTrigger aria-label="Mode"><SelectValue /></SelectTrigger>
					<SelectContent>
						{MODES.map((m) => <SelectItem key={m.mode} value={m.mode}>{m.label}</SelectItem>)}
					</SelectContent>
				</Select>
			</Field>

			<div className="flex flex-col gap-0.5">
				<Label className="text-xs text-muted-foreground">Category (from route mode)</Label>
				<span className="text-sm capitalize">{category}</span>
			</div>

			{showTth && (
				<Field label="Tth">
					<Select value={tth} onValueChange={(v) => setParams({ tth: v as TthFilter })}>
						<SelectTrigger aria-label="Tth"><SelectValue /></SelectTrigger>
						<SelectContent>
							{TTHS.map((t) => <SelectItem key={t} value={t}>{t}</SelectItem>)}
						</SelectContent>
					</Select>
				</Field>
			)}

			<Field label="Strategy">
				<Select value={strategy} onValueChange={(v) => setParams({ strategy: v as CalcStrategy })}>
					<SelectTrigger aria-label="Strategy"><SelectValue /></SelectTrigger>
					<SelectContent>
						{STRATEGIES.map((s) => <SelectItem key={s.strategy} value={s.strategy}>{s.label}</SelectItem>)}
					</SelectContent>
				</Select>
			</Field>

			{showRadius && (
				<Field label="Radius (m)">
					<Input type="number" aria-label="Radius" value={radius} min={1} onChange={(e) => setParams({ radius: Number(e.target.value) })} />
				</Field>
			)}

			{showS2 && (
				<>
					<Field label="S2 level">
						<Select value={String(s2Level)} onValueChange={(v) => setParams({ s2Level: Number(v) })}>
							<SelectTrigger aria-label="S2 level"><SelectValue /></SelectTrigger>
							<SelectContent>
								{S2_LEVELS.map((l) => <SelectItem key={l} value={String(l)}>Level {l}</SelectItem>)}
							</SelectContent>
						</Select>
					</Field>
					<Field label="S2 size">
						<Select value={String(s2Size)} onValueChange={(v) => setParams({ s2Size: Number(v) })}>
							<SelectTrigger aria-label="S2 size"><SelectValue /></SelectTrigger>
							<SelectContent>
								{S2_SIZES.map((s) => <SelectItem key={s} value={String(s)}>{s}x{s}</SelectItem>)}
							</SelectContent>
						</Select>
					</Field>
				</>
			)}

			{isCluster && (
				<Field label="Min points">
					<Input type="number" aria-label="Min points" value={minPoints} min={1} onChange={(e) => setParams({ minPoints: Number(e.target.value) })} />
				</Field>
			)}

			{showClusterKnobs && (
				<Field label="Clustering mode">
					<Select value={clusterMode ?? "default"} onValueChange={(v) => setParams({ clusterMode: v === "default" ? null : v })}>
						<SelectTrigger aria-label="Clustering mode"><SelectValue /></SelectTrigger>
						<SelectContent>
							<SelectItem value="default">Default</SelectItem>
							{(algorithms?.clustering ?? []).map((c) => <SelectItem key={c} value={c}>{c}</SelectItem>)}
						</SelectContent>
					</Select>
				</Field>
			)}

			{showMaxClusters && (
				<Field label="Max clusters (0 = unlimited)">
					<Input type="number" aria-label="Max clusters" value={maxClusters ?? 0} min={0} onChange={(e) => setParams({ maxClusters: Number(e.target.value) })} />
				</Field>
			)}

			{showClusterKnobs && (
				<div className="flex items-center justify-between">
					<Label htmlFor="center-clusters" className="text-xs text-muted-foreground">Center clusters</Label>
					<Switch id="center-clusters" aria-label="Center clusters" checked={centerClusters} onCheckedChange={(v) => setParams({ centerClusters: v })} />
				</div>
			)}

			<Field label="Routing algorithm">
				<Select value={sortBy ?? "default"} onValueChange={(v) => setParams({ sortBy: v === "default" ? null : v })}>
					<SelectTrigger aria-label="Routing algorithm"><SelectValue /></SelectTrigger>
					<SelectContent>
						<SelectItem value="default">Default (TSP)</SelectItem>
						<SelectItem value="unset">None</SelectItem>
						{(algorithms?.routing ?? []).map((r) => <SelectItem key={r} value={r}>{r}</SelectItem>)}
					</SelectContent>
				</Select>
			</Field>

			{job ? (
				<div className="flex flex-col gap-1">
					<div className="h-1.5 w-full overflow-hidden rounded bg-muted">
						<div className="h-full bg-primary transition-[width]" style={{ width: `${Math.round(job.progress * 100)}%` }} />
					</div>
					{job.phase && <span className="text-xs text-muted-foreground">{job.phase}</span>}
					{error && <span className="text-xs text-destructive">{error}</span>}
					{stats != null && <CalcStats stats={stats} />}
					<Button type="button" size="sm" variant="ghost" onClick={clear}>Clear</Button>
				</div>
			) : (
				<>
					<Button type="button" size="sm" disabled={blocked} onClick={onRun}>Calculate</Button>
					{blocked && disabledReason && <span className="text-xs text-muted-foreground">{disabledReason}</span>}
					{error && !blocked && <span className="text-xs text-destructive">{error}</span>}
				</>
			)}
		</div>
	);
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
	return (
		<div className="flex flex-col gap-1">
			<Label className="text-xs text-muted-foreground">{label}</Label>
			{children}
		</div>
	);
}

/** Render the couple of headline numbers from the job's Stats blob (best-effort). */
function CalcStats({ stats }: { stats: unknown }) {
	const s = stats as Record<string, unknown>;
	const total = typeof s?.total_clusters === "number" ? s.total_clusters : undefined;
	const distance = typeof s?.total_distance === "number" ? s.total_distance : undefined;
	if (total == null && distance == null) return null;
	return (
		<span className="text-xs text-muted-foreground">
			{total != null && <>{total} clusters</>}
			{distance != null && <> · {Math.round(distance)}m</>}
		</span>
	);
}
