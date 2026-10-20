import { useEffect, useMemo, useState } from "react";
import { useFormContext, useWatch } from "react-hook-form";
import { useGetList, useNotify, useRecordContext } from "ra-core";
import { Check, ChevronsUpDown, X } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
} from "@/components/ui/command";
import { Field, FieldLabel } from "@/components/ui/field";
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";
import { cn } from "@/lib/utils";
import { listWebhookAreas } from "@api";
import type { WebhookArea } from "@api";
import { groupAreaOptions, type AreaChoice } from "./dragonite-area-options";

interface DragoniteLink {
  webhook_id: number;
  dragonite_area_id: number | null;
}

interface Target {
  id: number;
  name: string;
  mode: string;
  active: boolean;
  project_id: number | null;
}

/** Whether a Dragonite target reaches a geofence in `projects`: it is active and
 *  either covers every geofence or the geofence is in its project. */
const reaches = (target: Target, projects: number[]) =>
  target.mode === "dragonite" &&
  target.active &&
  (target.project_id == null || projects.includes(Number(target.project_id)));

/** The target's Areas, fetched once per target. */
function useTargetAreas(targetId: number) {
  const [state, setState] = useState<
    { areas: WebhookArea[]; error: null } | { areas: null; error: string | null }
  >({ areas: null, error: null });
  useEffect(() => {
    let live = true;
    setState({ areas: null, error: null });
    listWebhookAreas(targetId).then(
      (areas) => live && setState({ areas, error: null }),
      (e) => live && setState({ areas: null, error: e instanceof Error ? e.message : String(e) }),
    );
    return () => {
      live = false;
    };
  }, [targetId]);
  return state;
}

/** One Dragonite target's Area picker. Its value lives in the form's
 *  `dragonite_links` array, which the write sends as the per-target changes. */
function AreaPicker({ target }: { target: Target }) {
  const record = useRecordContext();
  const notify = useNotify();
  const { getValues, setValue } = useFormContext();
  const links = (useWatch({ name: "dragonite_links" }) ?? []) as DragoniteLink[];
  const name = (useWatch({ name: "name" }) ?? "") as string;
  const mode = (useWatch({ name: "mode" }) ?? "unset") as string;
  const { areas, error } = useTargetAreas(target.id);
  const [open, setOpen] = useState(false);

  const value = links.find((l) => Number(l.webhook_id) === Number(target.id))?.dragonite_area_id ?? null;
  const { suggested, rest } = useMemo(
    () => groupAreaOptions(areas ?? [], { geofenceName: name, mode, selfId: record?.id }),
    [areas, name, mode, record?.id],
  );
  const selected = areas?.find((a) => a.id === value);

  const write = (areaId: number | null) => {
    const others = ((getValues("dragonite_links") ?? []) as DragoniteLink[]).filter(
      (l) => Number(l.webhook_id) !== Number(target.id),
    );
    setValue(
      "dragonite_links",
      [...others, { webhook_id: target.id, dragonite_area_id: areaId }],
      { shouldDirty: true },
    );
  };

  const pick = (choice: AreaChoice) => {
    setOpen(false);
    if (choice.holder) {
      notify(
        `Area "${choice.name}" is already linked to "${choice.holder.name}" on ${target.name}`,
        { type: "warning" },
      );
      return;
    }
    write(choice.id);
  };

  const item = (choice: AreaChoice) => (
    <CommandItem
      key={choice.id}
      value={String(choice.id)}
      keywords={[choice.name, choice.holder?.name ?? ""]}
      onSelect={() => pick(choice)}
    >
      <Check className={cn("mr-2 size-4", choice.id === value ? "opacity-100" : "opacity-0")} />
      <span>{choice.name}</span>
      {choice.holder && (
        <span className="ml-auto text-xs text-muted-foreground">
          held by {choice.holder.name}
        </span>
      )}
    </CommandItem>
  );

  const id = `dragonite-area-${target.id}`;
  return (
    <Field>
      <FieldLabel htmlFor={id}>Dragonite Area: {target.name}</FieldLabel>
      <div className="flex gap-2">
        <Popover open={open} onOpenChange={setOpen}>
          <PopoverTrigger asChild>
            <Button
              id={id}
              type="button"
              variant="outline"
              role="combobox"
              aria-expanded={open}
              disabled={areas === null}
              className="h-auto flex-1 justify-between py-1.75 font-normal"
            >
              {value != null ? (
                (selected?.name ?? `Area ${value}`)
              ) : (
                <span className="text-muted-foreground">
                  {error ? "Areas unavailable" : areas === null ? "Loading Areas..." : "No Area"}
                </span>
              )}
              <ChevronsUpDown className="shrink-0 opacity-50" />
            </Button>
          </PopoverTrigger>
          <PopoverContent className="w-full max-w-(--radix-popover-trigger-width) p-0">
            <Command>
              <CommandInput placeholder="Search..." />
              <CommandList>
                <CommandEmpty>No matching Area.</CommandEmpty>
                {suggested.length > 0 && (
                  <CommandGroup heading="Suggested">{suggested.map(item)}</CommandGroup>
                )}
                <CommandGroup heading="Areas">{rest.map(item)}</CommandGroup>
              </CommandList>
            </Command>
          </PopoverContent>
        </Popover>
        {value != null && (
          <Button
            type="button"
            variant="ghost"
            size="icon"
            aria-label={`Clear Area for ${target.name}`}
            onClick={() => write(null)}
          >
            <X />
          </Button>
        )}
      </div>
      {error && (
        <p className="text-sm text-destructive">Could not read the Areas of {target.name}: {error}</p>
      )}
    </Field>
  );
}

/** One Area picker per Dragonite target that reaches the geofence. Reach follows
 *  the form's `projects`, so on create the pickers appear as projects are chosen. */
export function GeofenceDragoniteAreas() {
  const { data: targets } = useGetList<Target>("webhook", {
    pagination: { page: 1, perPage: 1000 },
    sort: { field: "name", order: "ASC" },
    filter: {},
  });
  const projects = ((useWatch({ name: "projects" }) ?? []) as unknown[]).map(Number);
  const reaching = (targets ?? []).filter((t) => reaches(t, projects));
  if (reaching.length === 0) return null;
  return (
    <>
      {reaching.map((target) => (
        <AreaPicker key={target.id} target={target} />
      ))}
    </>
  );
}
