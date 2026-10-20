import type { WebhookArea, WebhookAreaHolder } from "@/api/types";

/** One Area in the picker, with the other geofence of the geofence's mode that
 *  holds it on the target (`null` when it is free). */
export interface AreaChoice {
  id: number;
  name: string;
  holder: WebhookAreaHolder | null;
}

const byName = (a: AreaChoice, b: AreaChoice) =>
  a.name.localeCompare(b.name, undefined, { sensitivity: "base" }) || a.id - b.id;

/** Split a target's Areas for the picker: the ones named like the geofence
 *  (ignoring case) go in `suggested` and are left out of `rest`; both are A to Z.
 *  An Area takes one geofence per mode, so only a holder of `mode` other than
 *  the geofence itself (`selfId`) counts. */
export function groupAreaOptions(
  areas: WebhookArea[],
  opts: { geofenceName: string; mode: string; selfId?: number | string | null },
): { suggested: AreaChoice[]; rest: AreaChoice[] } {
  const wanted = opts.geofenceName.toLowerCase();
  const suggested: AreaChoice[] = [];
  const rest: AreaChoice[] = [];
  for (const area of areas) {
    const choice: AreaChoice = {
      id: area.id,
      name: area.name,
      holder:
        area.held_by.find(
          (h) => h.mode === opts.mode && String(h.geofence) !== String(opts.selfId ?? ""),
        ) ?? null,
    };
    (wanted !== "" && area.name.toLowerCase() === wanted ? suggested : rest).push(choice);
  }
  return { suggested: suggested.sort(byName), rest: rest.sort(byName) };
}
