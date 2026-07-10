import { useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
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

export interface SaveDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  title: string;
  description?: string;
  /** Pre-selected mode when the dialog opens (the playground's current mode). */
  defaultMode: string;
  busy?: boolean;
  onConfirm: (name: string, mode: string) => void;
}

/** Small name + mode modal shared by the playground's "Save geofence" and
 *  "Save route" actions. Owns only its transient field state; the parent runs
 *  the actual create(s) in `onConfirm`. */
export function SaveDialog({
  open,
  onOpenChange,
  title,
  description,
  defaultMode,
  busy,
  onConfirm,
}: SaveDialogProps) {
  const [name, setName] = useState("");
  const [mode, setMode] = useState(defaultMode);

  // Reset fields whenever the dialog (re)opens so a prior entry doesn't linger.
  useEffect(() => {
    if (open) {
      setName("");
      setMode(defaultMode);
    }
  }, [open, defaultMode]);

  const trimmed = name.trim();
  const canSave = trimmed.length > 0 && !busy;
  const submit = () => {
    if (canSave) onConfirm(trimmed, mode);
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{title}</DialogTitle>
          {description ? (
            <DialogDescription>{description}</DialogDescription>
          ) : null}
        </DialogHeader>
        <div className="flex flex-col gap-3">
          <div className="flex flex-col gap-1">
            <Label htmlFor="save-name">Name</Label>
            <Input
              id="save-name"
              autoFocus
              value={name}
              onChange={(e) => setName(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") submit();
              }}
            />
          </div>
          <div className="flex flex-col gap-1">
            <Label htmlFor="save-mode">Mode</Label>
            <Select value={mode} onValueChange={setMode}>
              <SelectTrigger id="save-mode" aria-label="Mode" className="w-full">
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
        </div>
        <DialogFooter>
          <Button
            type="button"
            variant="ghost"
            onClick={() => onOpenChange(false)}
          >
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
