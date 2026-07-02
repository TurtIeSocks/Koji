import { useQuery } from "@tanstack/react-query";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Badge } from "@/components/ui/badge";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { useMapCalcStore } from "@/map/stores/map-calc-store";
import { useMapUIStore } from "@/map/stores/map-ui-store";
import { useMapViewStore } from "@/map/stores/map-view-store";
import { submitCalc, getAlgorithms } from "@/map/data/calc-client";
import {
  AREA_MODES,
  ROUTE_INPUT_MODES,
  boundsToAreaFC,
  featureToAreaFC,
  routeCoordsToClusters,
  buildCalcBody,
  type CalcMode,
} from "@/map/lib/calc-request";

const MODES: { mode: CalcMode; label: string }[] = [
  { mode: "cluster", label: "Cluster" },
  { mode: "route", label: "Route" },
  { mode: "bootstrap", label: "Bootstrap" },
  { mode: "reroute", label: "Reroute" },
  { mode: "routeStats", label: "Route stats" },
];
const CATEGORIES = ["pokestop", "gym", "spawnpoint", "station"];

export function CalcPanel() {
  const mode = useMapCalcStore((s) => s.mode);
  const category = useMapCalcStore((s) => s.category);
  const radius = useMapCalcStore((s) => s.radius);
  const minPoints = useMapCalcStore((s) => s.minPoints);
  const clusterMode = useMapCalcStore((s) => s.clusterMode);
  const areaSource = useMapCalcStore((s) => s.areaSource);
  const job = useMapCalcStore((s) => s.job);
  const stats = useMapCalcStore((s) => s.stats);
  const error = useMapCalcStore((s) => s.error);
  const setMode = useMapCalcStore((s) => s.setMode);
  const setCategory = useMapCalcStore((s) => s.setCategory);
  const setRadius = useMapCalcStore((s) => s.setRadius);
  const setMinPoints = useMapCalcStore((s) => s.setMinPoints);
  const setClusterMode = useMapCalcStore((s) => s.setClusterMode);
  const setAreaSource = useMapCalcStore((s) => s.setAreaSource);
  const startJob = useMapCalcStore((s) => s.startJob);
  const setError = useMapCalcStore((s) => s.setError);
  const clear = useMapCalcStore((s) => s.clear);

  const selectionKind = useMapUIStore((s) => s.selection.kind);
  // Clicking a geofence loads it into the editor → it IS the "selected geofence"
  // for area="selected" (its live-edited geometry is used).
  const editingGeofenceId = useMapUIStore((s) => s.editingGeofenceId);

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
  const showMinPoints = mode === "cluster" || mode === "route" || mode === "routeStats";

  const inFlight = !!job && (job.status === "queued" || job.status === "running");
  const needRoute = isRouteInput && selectionKind !== "route";
  const needGeofence = isAreaMode && areaSource === "selected" && !editingGeofenceId;
  const blocked = inFlight || needRoute || needGeofence;

  const handleCalculate = async () => {
    // Read transient inputs at click time (avoids subscribing to per-frame camera).
    const bounds = useMapViewStore.getState().settledBounds;
    const ui = useMapUIStore.getState();
    // route input → the selected route's coords; selected area → the geofence in
    // the editor (its live-edited geometry); else the current viewport bbox.
    const editedGeofence = ui.draftFeatures.features[0];
    const inputs = isRouteInput
      ? { clusters: routeCoordsToClusters(ui.selectedFeature) }
      : {
          area:
            areaSource === "selected" && editedGeofence
              ? featureToAreaFC(editedGeofence)
              : boundsToAreaFC(bounds),
        };
    const body = buildCalcBody({ mode, category, radius, minPoints, clusterMode }, inputs);
    try {
      const id = await submitCalc(body);
      startJob(id);
    } catch (e) {
      setError(e instanceof Error ? e.message : "failed to submit calc");
    }
  };

  return (
    <div className="flex w-64 flex-col gap-3 rounded-lg border bg-background/90 p-3 shadow-md backdrop-blur">
      <div className="flex items-center justify-between">
        <span className="text-sm font-medium">Calculate</span>
        {job && <Badge variant={job.status === "failed" ? "destructive" : "secondary"}>{job.status}</Badge>}
      </div>

      <Field label="Mode">
        <Select value={mode} onValueChange={(v) => setMode(v as CalcMode)}>
          <SelectTrigger aria-label="Mode"><SelectValue /></SelectTrigger>
          <SelectContent>
            {MODES.map((m) => <SelectItem key={m.mode} value={m.mode}>{m.label}</SelectItem>)}
          </SelectContent>
        </Select>
      </Field>

      {isAreaMode && (
        <>
          <Field label="Category">
            <Select value={category} onValueChange={setCategory}>
              <SelectTrigger aria-label="Category"><SelectValue /></SelectTrigger>
              <SelectContent>
                {CATEGORIES.map((c) => <SelectItem key={c} value={c} className="capitalize">{c}</SelectItem>)}
              </SelectContent>
            </Select>
          </Field>
          <Field label="Area">
            <Select value={areaSource} onValueChange={(v) => setAreaSource(v as "viewport" | "selected")}>
              <SelectTrigger aria-label="Area"><SelectValue /></SelectTrigger>
              <SelectContent>
                <SelectItem value="viewport">Current viewport</SelectItem>
                <SelectItem value="selected">Selected geofence</SelectItem>
              </SelectContent>
            </Select>
          </Field>
        </>
      )}

      {showClusterMode && algorithms?.clustering?.length ? (
        <Field label="Algorithm">
          <Select value={clusterMode ?? "default"} onValueChange={(v) => setClusterMode(v === "default" ? null : v)}>
            <SelectTrigger aria-label="Algorithm"><SelectValue /></SelectTrigger>
            <SelectContent>
              <SelectItem value="default">Default</SelectItem>
              {algorithms.clustering.map((c) => <SelectItem key={c} value={c}>{c}</SelectItem>)}
            </SelectContent>
          </Select>
        </Field>
      ) : null}

      <Field label="Radius (m)">
        <Input type="number" aria-label="Radius" value={radius} min={1} onChange={(e) => setRadius(Number(e.target.value))} />
      </Field>
      {showMinPoints && (
        <Field label="Min points">
          <Input type="number" aria-label="Min points" value={minPoints} min={1} onChange={(e) => setMinPoints(Number(e.target.value))} />
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
          <Button size="sm" variant="ghost" onClick={clear}>Clear</Button>
        </div>
      ) : (
        <>
          <Button size="sm" disabled={blocked} onClick={handleCalculate}>Calculate</Button>
          {needRoute && <span className="text-xs text-muted-foreground">Select a route first.</span>}
          {needGeofence && <span className="text-xs text-muted-foreground">Select a geofence first.</span>}
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
