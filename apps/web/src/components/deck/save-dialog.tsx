import { useEffect, useState } from "react";
import { useGetList } from "shadmin-core";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { GEOFENCE_MODES } from "@/lib/constants";

export interface SavePayload {
  geofence: { name: string; mode: string; parent?: number };
  /** Present only when a calc result exists and the route section is filled. */
  route?: { name: string };
}

export interface SaveDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** A calc result exists → also offer to save it as a route (second section). */
  hasRoute: boolean;
  /** Pre-selected geofence mode (the playground's current data mode). */
  defaultMode: string;
  busy?: boolean;
  onConfirm: (payload: SavePayload) => void;
}

const NONE = "none";

/** Save the playground drawing. A Geofence section (name / mode / optional
 *  parent) always; a Route section only when a calc result exists — that route
 *  is auto-linked to the geofence saved here, so it has no fence picker. */
export function SaveDialog({
  open,
  onOpenChange,
  hasRoute,
  defaultMode,
  busy,
  onConfirm,
}: SaveDialogProps) {
  const [gName, setGName] = useState("");
  const [gMode, setGMode] = useState(defaultMode);
  const [gParent, setGParent] = useState(NONE);
  const [rName, setRName] = useState("");

  // Reset fields whenever the dialog (re)opens so a prior entry doesn't linger.
  useEffect(() => {
    if (open) {
      setGName("");
      setGMode(defaultMode);
      setGParent(NONE);
      setRName("");
    }
  }, [open, defaultMode]);

  // Parent options — every geofence, name-sorted. Only fetched while open.
  const { data: fences, isPending } = useGetList(
    "geofence",
    { pagination: { page: 1, perPage: 1000 }, sort: { field: "name", order: "ASC" } },
    { enabled: open },
  );

  const name = gName.trim();
  const canSave = name.length > 0 && !busy;
  const submit = () => {
    if (!canSave) return;
    onConfirm({
      geofence: {
        name,
        mode: gMode,
        parent: gParent === NONE ? undefined : Number(gParent),
      },
      route: hasRoute ? { name: rName.trim() || name } : undefined,
    });
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Save</DialogTitle>
        </DialogHeader>

        <section className="flex flex-col gap-3">
          <h3 className="text-sm font-medium">Geofence</h3>
          <div className="flex flex-col gap-1">
            <Label htmlFor="gf-name">Name</Label>
            <Input
              id="gf-name"
              autoFocus
              value={gName}
              onChange={(e) => setGName(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") submit();
              }}
            />
          </div>
          <div className="flex flex-col gap-1">
            <Label htmlFor="gf-mode">Mode</Label>
            <Select value={gMode} onValueChange={setGMode}>
              <SelectTrigger id="gf-mode" aria-label="Mode" className="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {GEOFENCE_MODES.map((m) => (
                  <SelectItem key={m.id} value={m.id}>
                    {m.name}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
          <div className="flex flex-col gap-1">
            <Label htmlFor="gf-parent">Parent</Label>
            <Select value={gParent} onValueChange={setGParent} disabled={isPending}>
              <SelectTrigger id="gf-parent" aria-label="Parent" className="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value={NONE}>None</SelectItem>
                {(fences ?? []).map((f) => (
                  <SelectItem key={f.id} value={String(f.id)}>
                    {f.name}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
        </section>

        {hasRoute ? (
          <section className="flex flex-col gap-3 border-t pt-3">
            <h3 className="text-sm font-medium">Route</h3>
            <div className="flex flex-col gap-1">
              <Label htmlFor="rt-name">Name</Label>
              <Input
                id="rt-name"
                value={rName}
                placeholder={name || "same as geofence"}
                onChange={(e) => setRName(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") submit();
                }}
              />
            </div>
            <p className="text-xs text-muted-foreground">
              This route will be automatically associated with the geofence above.
            </p>
          </section>
        ) : null}

        <DialogFooter>
          <Button type="button" variant="ghost" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button type="button" disabled={!canSave} onClick={submit}>
            {busy ? "Saving…" : "Save"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
