import { useEffect, useMemo, useState } from "react";
import { useNotify } from "shadmin-core";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { publishGeofences } from "@api";
import type { PublishPlan, PublishSkip, PublishWrite } from "@/api/types";

/** Rows drawn per group before "Show more", so a 2,000-fence plan stays a few
 *  hundred DOM nodes. */
const PAGE = 50;

const SKIP_TITLES: Record<PublishSkip["reason"], string> = {
  no_target: "No Dragonite target reaches these",
  no_link: "The target that reaches these holds no link",
  not_found: "No such geofence",
};

const plural = (n: number, one: string, many = `${one}s`) =>
  `${n} ${n === 1 ? one : many}`;

interface Group<T> {
  key: string;
  title: string;
  rows: T[];
}

/** Group `rows` by `keyOf`, keeping first-seen order. */
function groupBy<T>(
  rows: T[],
  keyOf: (row: T) => string,
  titleOf: (row: T) => string,
): Group<T>[] {
  const groups = new Map<string, Group<T>>();
  for (const row of rows) {
    const key = keyOf(row);
    let group = groups.get(key);
    if (!group) {
      group = { key, title: titleOf(row), rows: [] };
      groups.set(key, group);
    }
    group.rows.push(row);
  }
  return [...groups.values()];
}

function GroupList<T>({
  group,
  render,
}: {
  group: Group<T>;
  render: (row: T) => React.ReactNode;
}) {
  const [shown, setShown] = useState(PAGE);
  const more = group.rows.length - shown;
  return (
    <section className="flex flex-col gap-1" data-testid="publish-group">
      <h3 className="text-sm font-medium">
        {group.title}{" "}
        <span className="text-muted-foreground">({group.rows.length})</span>
      </h3>
      <ul className="flex flex-col gap-0.5 text-sm">
        {group.rows.slice(0, shown).map(render)}
      </ul>
      {more > 0 && (
        <Button
          type="button"
          size="sm"
          variant="ghost"
          className="self-start"
          onClick={() => setShown(shown + PAGE * 4)}
        >
          Show {Math.min(more, PAGE * 4)} more
        </Button>
      )}
    </section>
  );
}

const writeRow = (w: PublishWrite) => (
  <li key={`${w.geofence}:${w.webhook_id}`} className="flex gap-2">
    <span className="font-medium">{w.name}</span>
    <span className="text-muted-foreground">{w.mode}</span>
    <span className="ml-auto">
      Area {w.dragonite_area_name ? `${w.dragonite_area_name} ` : ""}(id{" "}
      {w.dragonite_area_id})
    </span>
  </li>
);

const skipRow = (s: PublishSkip) => (
  <li key={s.geofence} title={s.message} className="flex gap-2">
    <span className="font-medium">{s.name ?? `#${s.geofence}`}</span>
    {s.mode && <span className="text-muted-foreground">{s.mode}</span>}
  </li>
);

/**
 * Asks the server what publishing `ids` would write (a dry run), lists it, and
 * sends exactly those writes on Confirm. Mount it when the admin clicks
 * Publish: it plans on mount. Cancel (or closing) sends nothing.
 */
export function PublishDialog({
  ids: requested,
  onClose,
  onPublished,
}: {
  ids: Array<string | number>;
  onClose: () => void;
  onPublished?: () => void;
}) {
  const notify = useNotify();
  // The ids asked about when the dialog opened: a re-render with a new array
  // must not re-plan under the admin's eyes.
  const [ids] = useState(requested);
  const [plan, setPlan] = useState<PublishPlan | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [sending, setSending] = useState(false);

  useEffect(() => {
    let live = true;
    publishGeofences(ids, { dryRun: true }).then(
      (p) => live && setPlan(p),
      (e) =>
        live &&
        setError(e instanceof Error ? e.message : "Could not plan the publish"),
    );
    return () => {
      live = false;
    };
  }, [ids]);

  const targets = useMemo(
    () =>
      groupBy(
        plan?.writes ?? [],
        (w) => String(w.webhook_id),
        (w) => `${w.webhook_name} (id ${w.webhook_id})`,
      ),
    [plan],
  );
  const skipped = useMemo(
    () =>
      groupBy(
        plan?.skipped ?? [],
        (s) => s.reason,
        (s) => SKIP_TITLES[s.reason],
      ),
    [plan],
  );
  const sendIds = useMemo(
    () => [...new Set((plan?.writes ?? []).map((w) => w.geofence))],
    [plan],
  );

  const confirm = async () => {
    setSending(true);
    try {
      const sent = await publishGeofences(sendIds, { dryRun: false });
      notify(
        `${plural(sent.writes.length, "write")} queued for ${plural(
          new Set(sent.writes.map((w) => w.geofence)).size,
          "geofence",
        )}`,
        { type: "info" },
      );
      onPublished?.();
      onClose();
    } catch (e) {
      notify(e instanceof Error ? e.message : "Publish failed", {
        type: "error",
      });
      setSending(false);
    }
  };

  const writes = plan?.writes.length ?? 0;
  return (
    <Dialog open onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="max-h-[85vh] overflow-y-auto sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle>Publish {plural(ids.length, "geofence")}</DialogTitle>
          <DialogDescription>
            {plan
              ? `${plural(writes, "write")} to ${plural(targets.length, "Dragonite target")}, ${plural(
                  plan.skipped.length,
                  "geofence",
                )} skipped. Confirm sends exactly these writes.`
              : (error ?? "Checking where each geofence goes…")}
          </DialogDescription>
        </DialogHeader>
        {plan && (
          <div className="flex flex-col gap-4">
            {targets.map((g) => (
              <GroupList key={g.key} group={g} render={writeRow} />
            ))}
            {skipped.map((g) => (
              <GroupList key={g.key} group={g} render={skipRow} />
            ))}
            {writes === 0 && (
              <p className="text-sm text-muted-foreground">
                Nothing to send.
              </p>
            )}
          </div>
        )}
        <DialogFooter>
          <Button type="button" variant="outline" onClick={onClose}>
            Cancel
          </Button>
          <Button
            type="button"
            disabled={!plan || writes === 0 || sending}
            onClick={confirm}
          >
            Confirm
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
