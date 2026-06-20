import { useEffect, useMemo, useState } from "react";
import { useFormContext } from "react-hook-form";
import { Link } from "react-router";
import { postImport, type ImportResult } from "@/lib/import-api";
import { featuresToImportItems } from "../to-import-items";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";

interface ReviewStepProps {
  onCommitted: () => void;
}

type ActionVariant = "default" | "secondary" | "outline" | "destructive";

function actionVariant(action: string): ActionVariant {
  switch (action) {
    case "create": return "default";
    case "update": return "secondary";
    case "skip": return "outline";
    case "fail": return "destructive";
    default: return "outline";
  }
}

export function ReviewStep({ onCommitted }: ReviewStepProps) {
  const { getValues } = useFormContext();
  const [report, setReport] = useState<ImportResult | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [done, setDone] = useState<ImportResult | null>(null);

  // Serialize the form features ONCE when the step mounts, so the dry-run
  // preview and the real commit send byte-identical payloads. Steps are
  // conditionally rendered, so the step remounts on revisit and re-reads the
  // latest features then — re-reading per call would not reflect edits anyway.
  const items = useMemo(
    () =>
      featuresToImportItems(
        getValues("features") as Parameters<typeof featuresToImportItems>[0],
      ),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [],
  );

  async function runDryRun() {
    setLoading(true);
    setError(null);
    try {
      const result = await postImport({ dry_run: true, items });
      setReport(result);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }

  // eslint-disable-next-line react-hooks/exhaustive-deps
  useEffect(() => { void runDryRun(); }, []);

  async function handleCommit() {
    if (!report) return;
    setLoading(true);
    setError(null);
    try {
      const result = await postImport({ dry_run: false, items });
      if (result.committed) {
        setDone(result);
        onCommitted();
      } else {
        // Backend rolled everything back — show updated report with failed rows
        setReport(result);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }

  if (done) {
    const s = done.summary;
    return (
      <div className="flex flex-col gap-4">
        <p className="text-lg font-semibold text-green-600">Import committed!</p>
        <p className="text-sm text-muted-foreground">
          Created: {s.create} · Updated: {s.update} · Skipped: {s.skip}
        </p>
        <Button asChild variant="outline">
          <Link to="/geofences">Back to geofences</Link>
        </Button>
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-4">
      <h2 className="text-base font-semibold">Review import</h2>

      {loading && <p className="text-sm text-muted-foreground">Checking…</p>}

      {error && (
        <p role="alert" className="text-sm text-destructive">
          {error}
        </p>
      )}

      {report && !loading && (
        <>
          <div className="flex gap-4 text-sm">
            <span>Create: <strong>{report.summary.create}</strong></span>
            <span>Update: <strong>{report.summary.update}</strong></span>
            <span>Skip: <strong>{report.summary.skip}</strong></span>
            <span>Fail: <strong>{report.summary.fail}</strong></span>
          </div>

          {/* ponytail: plain table, DataTable is overkill for a wizard step */}
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b text-left text-muted-foreground">
                <th className="pb-1 pr-4">#</th>
                <th className="pb-1 pr-4">Name</th>
                <th className="pb-1 pr-4">Action</th>
                <th className="pb-1">Reason</th>
              </tr>
            </thead>
            <tbody>
              {report.results.map((row) => (
                <tr key={row.index} className="border-b last:border-0">
                  <td className="py-1 pr-4">{row.index}</td>
                  <td className="py-1 pr-4">{row.name}</td>
                  <td className="py-1 pr-4">
                    <Badge variant={actionVariant(row.action)}>{row.action}</Badge>
                  </td>
                  <td className="py-1 text-muted-foreground">{row.reason ?? ""}</td>
                </tr>
              ))}
            </tbody>
          </table>

          <div className="flex gap-2">
            <Button
              variant="outline"
              onClick={() => void runDryRun()}
              disabled={loading}
            >
              Re-check
            </Button>
            <Button
              onClick={() => void handleCommit()}
              disabled={report == null || loading || report.summary.fail > 0}
            >
              Commit import
            </Button>
          </div>
        </>
      )}
    </div>
  );
}
