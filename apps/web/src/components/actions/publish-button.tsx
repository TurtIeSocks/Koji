import { Send } from "lucide-react";
import {
  useNotify,
  useRefresh,
  useResourceContext,
  useRecordContext,
  useListContext,
  useUnselectAll,
} from "shadmin-core";
import { Button } from "@/components/ui/button";
import { internalFetch } from "@/lib/http";

/** Maps ra-core resource name → /internal path segment for publish. */
const PUBLISH_SEG: Record<string, string> = {
  geofence: "geofences",
  route: "routes",
};

const segFor = (resource: string): string => PUBLISH_SEG[resource] ?? resource;

/**
 * Calls POST /internal/{seg}/{id}/publish for the current record.
 * - 2xx → info notification + refresh
 * - 422 → warning notification (no linked Dragonite area)
 * - other error → error notification
 *
 * Reads record + resource from context. Drop into any record-scoped toolbar.
 */
export function PublishButton() {
  const resource = useResourceContext();
  const record = useRecordContext();
  const notify = useNotify();
  const refresh = useRefresh();

  const handleClick = async (e: React.MouseEvent) => {
    e.stopPropagation();
    if (!record) return;
    try {
      const res = await internalFetch(
        `/${segFor(resource ?? "")}/${record.id}/publish`,
        { method: "POST" },
      );
      if (res.status === 422) {
        const body = res.json as { error?: string } | null;
        notify(body?.error ?? "Cannot publish: no linked Dragonite area", {
          type: "warning",
        });
        return;
      }
      if (res.status < 200 || res.status >= 300) {
        notify(`Publish failed (${res.status})`, { type: "error" });
        return;
      }
      notify("Published", { type: "info" });
      refresh();
    } catch (err) {
      notify(err instanceof Error ? err.message : "Publish failed", {
        type: "error",
      });
    }
  };

  return (
    <Button size="sm" variant="secondary" type="button" onClick={handleClick}>
      <Send />
      Publish
    </Button>
  );
}

/**
 * Bulk-action variant. Loops `Promise.allSettled` over selected ids, then
 * summarises how many succeeded and how many failed/422'd.
 *
 * Reads selectedIds + resource from list context.
 */
export function BulkPublishButton() {
  const resource = useResourceContext();
  const { selectedIds } = useListContext();
  const unselectAll = useUnselectAll(resource ?? "");
  const notify = useNotify();
  const refresh = useRefresh();

  const handleClick = async (e: React.MouseEvent) => {
    e.stopPropagation();
    const seg = segFor(resource ?? "");
    const results = await Promise.allSettled(
      selectedIds.map((id) =>
        internalFetch(`/${seg}/${id}/publish`, { method: "POST" }),
      ),
    );

    let succeeded = 0;
    let noArea = 0;
    let failed = 0;

    for (const r of results) {
      if (r.status === "fulfilled") {
        if (r.value.status === 422) noArea++;
        else if (r.value.status >= 200 && r.value.status < 300) succeeded++;
        else failed++;
      } else {
        failed++;
      }
    }

    const parts: string[] = [];
    if (succeeded > 0) parts.push(`${succeeded} published`);
    if (noArea > 0) parts.push(`${noArea} skipped (no Dragonite area)`);
    if (failed > 0) parts.push(`${failed} failed`);

    const notifyType =
      failed > 0 ? "error" : noArea > 0 ? "warning" : "info";
    notify(parts.join(", ") || "Done", { type: notifyType });

    unselectAll();
    refresh();
  };

  return (
    <Button size="sm" variant="secondary" type="button" onClick={handleClick}>
      <Send />
      Publish
    </Button>
  );
}
