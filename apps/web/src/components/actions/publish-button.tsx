import { useState } from "react";
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
import { PublishDialog } from "./publish-dialog";

/**
 * Publish for the current geofence: opens a dialog listing where it goes
 * (Dragonite target and Area) and sends only on Confirm.
 *
 * Reads the record from context. Drop into any geofence record toolbar.
 */
export function PublishButton() {
  const record = useRecordContext();
  const refresh = useRefresh();
  const [open, setOpen] = useState(false);

  return (
    <>
      <Button
        size="sm"
        variant="secondary"
        type="button"
        disabled={!record}
        onClick={(e) => {
          e.stopPropagation();
          setOpen(true);
        }}
      >
        <Send />
        Publish
      </Button>
      {open && record && (
        <PublishDialog
          ids={[record.id]}
          onClose={() => setOpen(false)}
          onPublished={refresh}
        />
      )}
    </>
  );
}

/**
 * Bulk Publish for the selected geofences: the same dialog, listing every
 * write grouped by Dragonite target before anything is sent.
 *
 * Reads selectedIds + resource from list context.
 */
export function BulkPublishButton() {
  const resource = useResourceContext();
  const { selectedIds } = useListContext();
  const unselectAll = useUnselectAll(resource ?? "");
  const refresh = useRefresh();
  const [open, setOpen] = useState(false);

  return (
    <>
      <Button
        size="sm"
        variant="secondary"
        type="button"
        onClick={(e) => {
          e.stopPropagation();
          setOpen(true);
        }}
      >
        <Send />
        Publish
      </Button>
      {open && (
        <PublishDialog
          ids={selectedIds}
          onClose={() => setOpen(false)}
          onPublished={() => {
            unselectAll();
            refresh();
          }}
        />
      )}
    </>
  );
}

/**
 * Bulk publish for routes, which still send straight away. Loops `Promise.allSettled` over selected ids, then
 * summarises how many succeeded and how many failed/422'd.
 *
 * Reads selectedIds + resource from list context.
 */
export function BulkRoutePublishButton() {
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
