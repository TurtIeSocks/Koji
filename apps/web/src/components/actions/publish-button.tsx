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
import { publishRecord } from "@api";

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
      const result = await publishRecord(resource ?? "", record.id);
      if (!result.ok) {
        notify(result.message, { type: result.warning ? "warning" : "error" });
        return;
      }
      notify(result.message, { type: "info" });
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
    const results = await Promise.allSettled(
      selectedIds.map((id) => publishRecord(resource ?? "", id)),
    );

    let succeeded = 0;
    let noArea = 0;
    let failed = 0;

    for (const r of results) {
      if (r.status === "fulfilled") {
        if (r.value.warning) noArea++;
        else if (r.value.ok) succeeded++;
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
