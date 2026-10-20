import { useRef, useState, type ReactNode } from "react";
import { ChevronRight } from "lucide-react";
import { useNotify, useRecordContext } from "ra-core";
import { Button } from "@/components/ui/button";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { linkDragonite } from "@api";
import type { LinkDragonitePlan } from "@api";

/** Rows rendered per section before "Show more", so a plan of thousands never
 *  puts thousands of nodes on the page. */
const PAGE = 100;

type State =
  | { step: "closed" }
  | { step: "loading" }
  | { step: "error"; message: string }
  | { step: "preview"; plan: LinkDragonitePlan }
  | { step: "applying"; plan: LinkDragonitePlan }
  | { step: "applied"; plan: LinkDragonitePlan };

const message = (e: unknown) => (e instanceof Error ? e.message : String(e));

const Section = <T,>({
  title,
  rows,
  row,
}: {
  title: string;
  rows: T[];
  row: (r: T) => ReactNode;
}) => {
  const [shown, setShown] = useState(PAGE);
  return (
    <Collapsible className="rounded-md border">
      <CollapsibleTrigger
        disabled={rows.length === 0}
        className="group flex w-full items-center gap-2 px-3 py-2 text-left text-sm font-medium disabled:opacity-60"
      >
        <ChevronRight className="size-4 transition-transform group-data-[state=open]:rotate-90" />
        <span>{title}</span>
        <span className="ml-auto tabular-nums text-muted-foreground">{rows.length}</span>
      </CollapsibleTrigger>
      <CollapsibleContent>
        <ul className="max-h-60 divide-y overflow-y-auto border-t text-sm">
          {rows.slice(0, shown).map((r, i) => (
            <li key={i} data-slot="link-row" className="px-3 py-1.5">
              {row(r)}
            </li>
          ))}
          {rows.length > shown && (
            <li className="px-3 py-1.5">
              <Button
                type="button"
                variant="ghost"
                size="sm"
                onClick={() => setShown((n) => n + PAGE)}
              >
                Show more ({rows.length - shown} left)
              </Button>
            </li>
          )}
        </ul>
      </CollapsibleContent>
    </Collapsible>
  );
};

const Plan = ({ plan, applied }: { plan: LinkDragonitePlan; applied: boolean }) => (
  <div className="flex max-h-[60vh] flex-col gap-2 overflow-y-auto">
    <Section
      title={applied ? "Linked" : "Will link"}
      rows={plan.linked}
      row={(r) => (
        <>
          {r.name} <span className="text-muted-foreground">({r.mode}) to Area {r.dragonite_area_id}</span>
        </>
      )}
    />
    <Section
      title="Already linked"
      rows={plan.already_linked}
      row={(r) => (
        <>
          {r.name} <span className="text-muted-foreground">({r.mode}) on Area {r.dragonite_area_id}</span>
        </>
      )}
    />
    <Section
      title="Conflicts"
      rows={plan.conflicts}
      row={(r) => (
        <>
          {r.name}{" "}
          <span className="text-muted-foreground">
            ({r.mode}) stays on Area {r.dragonite_area_id}, its name matches Area{" "}
            {r.wanted_dragonite_area_id}
          </span>
        </>
      )}
    />
    <Section
      title="Area already taken"
      rows={plan.area_taken}
      row={(r) => (
        <>
          {r.name}{" "}
          <span className="text-muted-foreground">
            ({r.mode}) not linked, Area {r.dragonite_area_id} is held by geofence {r.held_by}
          </span>
        </>
      )}
    />
    <Section
      title="Areas without a geofence"
      rows={plan.areas_without_geofence}
      row={(r) => (
        <>
          {r.name} <span className="text-muted-foreground">(Area {r.dragonite_area_id})</span>
        </>
      )}
    />
    <Section
      title="Geofences without an Area"
      rows={plan.geofences_without_area}
      row={(r) => (
        <>
          {r.name} <span className="text-muted-foreground">({r.mode})</span>
        </>
      )}
    />
  </div>
);

const LinkByName = ({ webhookId }: { webhookId: string | number }) => {
  const notify = useNotify();
  const [state, setState] = useState<State>({ step: "closed" });
  // Bumped on every open/close so an answer that lands after the dialog was
  // closed (or reopened) is dropped.
  const run = useRef(0);

  const open = async () => {
    const mine = ++run.current;
    setState({ step: "loading" });
    try {
      const plan = await linkDragonite(webhookId, { dryRun: true });
      if (run.current === mine) setState({ step: "preview", plan });
    } catch (e) {
      if (run.current === mine) setState({ step: "error", message: message(e) });
    }
  };

  const close = () => {
    run.current++;
    setState({ step: "closed" });
  };

  const apply = async (plan: LinkDragonitePlan) => {
    const mine = ++run.current;
    setState({ step: "applying", plan });
    try {
      const done = await linkDragonite(webhookId, { dryRun: false });
      if (run.current !== mine) return;
      setState({ step: "applied", plan: done });
      notify(`Linked ${done.linked.length} geofences`, { type: "success" });
    } catch (e) {
      if (run.current !== mine) return;
      setState({ step: "preview", plan });
      notify(`Link by name failed: ${message(e)}`, { type: "error" });
    }
  };

  return (
    <>
      <Button type="button" variant="outline" onClick={open}>
        Link by name
      </Button>
      {/* Closing while Apply runs would drop its answer after the server has
          already linked, so the dialog stays until it lands. */}
      <Dialog
        open={state.step !== "closed"}
        onOpenChange={(o) => !o && state.step !== "applying" && close()}
      >
        <DialogContent
          className="sm:max-w-2xl"
          showCloseButton={state.step !== "applying"}
        >
          <DialogHeader>
            <DialogTitle>Link by name</DialogTitle>
            <DialogDescription>
              Links each Area to the geofences with exactly its name, in every mode, among the
              geofences this webhook reaches.
            </DialogDescription>
          </DialogHeader>
          {state.step === "loading" && <p className="text-sm">Reading the plan…</p>}
          {state.step === "error" && (
            <p role="alert" className="text-sm text-destructive">
              {state.message}
            </p>
          )}
          {(state.step === "preview" || state.step === "applying") && (
            <Plan plan={state.plan} applied={false} />
          )}
          {state.step === "applied" && (
            <>
              <p className="text-sm font-medium">Linked {state.plan.linked.length} geofences</p>
              <Plan plan={state.plan} applied />
            </>
          )}
          <DialogFooter>
            {(state.step === "preview" || state.step === "applying") && (
              <>
                <Button type="button" variant="outline" onClick={close} disabled={state.step === "applying"}>
                  Cancel
                </Button>
                <Button
                  type="button"
                  onClick={() => apply(state.plan)}
                  disabled={state.step === "applying" || state.plan.linked.length === 0}
                >
                  {state.step === "applying" ? "Applying…" : "Apply"}
                </Button>
              </>
            )}
            {(state.step === "applied" || state.step === "error") && (
              <Button type="button" variant="outline" onClick={close}>
                Done
              </Button>
            )}
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
};

/** The Link by name action of a Dragonite webhook's page; nothing on other
 *  modes. */
export const WebhookLinkByName = () => {
  const record = useRecordContext();
  if (!record || record.mode !== "dragonite") return null;
  return <LinkByName webhookId={record.id} />;
};
