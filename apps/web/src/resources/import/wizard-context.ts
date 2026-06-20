import { useState } from "react";

export const IMPORT_STEPS = ["Source", "Map & Name", "Assign", "Review"] as const;

/** Active-step state for the import wizard. */
export function useImportStep() {
  const [active, setActive] = useState<number>(0);
  const next = () =>
    setActive((s) => Math.min(s + 1, IMPORT_STEPS.length - 1));
  const back = () => setActive((s) => Math.max(s - 1, 0));
  return { active, setActive, next, back };
}
