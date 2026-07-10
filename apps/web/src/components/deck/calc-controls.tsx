import { useQuery } from "@tanstack/react-query";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Badge } from "@/components/ui/badge";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { getAlgorithms } from "@/map/data/calc-client";
import { AREA_MODES, ROUTE_INPUT_MODES, type CalcMode } from "@/map/lib/calc-request";
import type { UseCalcReturn } from "./use-calc";

const MODES: { mode: CalcMode; label: string }[] = [
	{ mode: "cluster", label: "Cluster" },
	{ mode: "route", label: "Route" },
	{ mode: "bootstrap", label: "Bootstrap" },
	{ mode: "reroute", label: "Reroute" },
	{ mode: "routeStats", label: "Route stats" },
];
const CATEGORIES = ["pokestop", "gym", "spawnpoint", "station"];

export interface CalcControlsProps {
	calc: UseCalcReturn;
	onRun: () => void;
	disabled?: boolean;
	disabledReason?: string;
}

/** Presentational calc panel — driven entirely by the `calc` prop (Task 9's
 *  `useCalc`), no global store. The parent owns area/route inputs and gating
 *  (`disabled`/`disabledReason`); this component only renders params/progress
 *  and calls `onRun` on submit. */
export function CalcControls({ calc, onRun, disabled, disabledReason }: CalcControlsProps) {
	const { params, setParams, job, stats, error, clear } = calc;
	const { mode, category, radius, minPoints, clusterMode, sortBy } = params;

	// Cluster-algorithm options (cluster/route only). If the endpoint is down the
	// select just hides and the server default applies.
	const { data: algorithms } = useQuery({
		queryKey: ["calc", "algorithms"],
		queryFn: getAlgorithms,
		staleTime: Infinity,
	});

	const isAreaMode = AREA_MODES.includes(mode);
	const isRouteInput = ROUTE_INPUT_MODES.includes(mode);
	const showClusterMode = mode === "cluster" || mode === "route";
	const showSortBy = mode === "route" || mode === "reroute";
	const showMinPoints = mode === "cluster" || mode === "route" || mode === "routeStats";

	const blocked = !!disabled;

	return (
		<div className="flex w-64 flex-col gap-3 rounded-lg border bg-background/90 p-3 shadow-md backdrop-blur">
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

			{isAreaMode && (
				<Field label="Category">
					<Select value={category} onValueChange={(v) => setParams({ category: v })}>
						<SelectTrigger aria-label="Category"><SelectValue /></SelectTrigger>
						<SelectContent>
							{CATEGORIES.map((c) => <SelectItem key={c} value={c} className="capitalize">{c}</SelectItem>)}
						</SelectContent>
					</Select>
				</Field>
			)}

			{showClusterMode && algorithms?.clustering?.length ? (
				<Field label="Algorithm">
					<Select value={clusterMode ?? "default"} onValueChange={(v) => setParams({ clusterMode: v === "default" ? null : v })}>
						<SelectTrigger aria-label="Algorithm"><SelectValue /></SelectTrigger>
						<SelectContent>
							<SelectItem value="default">Default</SelectItem>
							{algorithms.clustering.map((c) => <SelectItem key={c} value={c}>{c}</SelectItem>)}
						</SelectContent>
					</Select>
				</Field>
			) : null}

			{showSortBy && algorithms?.routing?.length ? (
				<Field label="Routing (sort)">
					<Select value={sortBy ?? "default"} onValueChange={(v) => setParams({ sortBy: v === "default" ? null : v })}>
						<SelectTrigger aria-label="Routing"><SelectValue /></SelectTrigger>
						<SelectContent>
							<SelectItem value="default">Default (TSP)</SelectItem>
							{algorithms.routing.map((r) => <SelectItem key={r} value={r}>{r}</SelectItem>)}
						</SelectContent>
					</Select>
				</Field>
			) : null}

			<Field label="Radius (m)">
				<Input type="number" aria-label="Radius" value={radius} min={1} onChange={(e) => setParams({ radius: Number(e.target.value) })} />
			</Field>
			{showMinPoints && (
				<Field label="Min points">
					<Input type="number" aria-label="Min points" value={minPoints} min={1} onChange={(e) => setParams({ minPoints: Number(e.target.value) })} />
				</Field>
			)}

			{isRouteInput && (
				<p className="text-xs text-muted-foreground">Uses the selected route as input.</p>
			)}

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
