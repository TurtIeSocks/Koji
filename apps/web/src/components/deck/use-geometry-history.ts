import { useCallback, useEffect, useReducer, useRef } from "react";
import { useFormContext, useWatch } from "react-hook-form";

export interface HistoryControls {
  undo: () => void;
  redo: () => void;
  canUndo: boolean;
  canRedo: boolean;
}

/** Undo/redo for a single RHF geometry field. Snapshots the value on every
 *  genuine change (each committed draw/edit) and restores via `setValue`, which
 *  re-hydrates the deck draft. A `restoring` flag keeps the restore itself from
 *  being recorded as a new step. Playground-scoped — the embedded workbench maps
 *  don't wire this up. */
export function useGeometryHistory(source: string): HistoryControls {
  const { setValue, getValues } = useFormContext();
  const value = useWatch({ name: source });
  const past = useRef<unknown[]>([]);
  const future = useRef<unknown[]>([]);
  const last = useRef<unknown>(getValues(source));
  const restoring = useRef(false);
  const [, bump] = useReducer((x: number) => x + 1, 0);

  useEffect(() => {
    if (restoring.current) {
      restoring.current = false;
      last.current = value;
      return;
    }
    if (JSON.stringify(value) === JSON.stringify(last.current)) return;
    past.current.push(last.current);
    future.current = [];
    last.current = value;
    bump();
  }, [value]);

  const undo = useCallback(() => {
    if (past.current.length === 0) return;
    const prev = past.current.pop();
    future.current.push(last.current);
    restoring.current = true;
    last.current = prev;
    setValue(source, prev, { shouldDirty: true });
    bump();
  }, [setValue, source]);

  const redo = useCallback(() => {
    if (future.current.length === 0) return;
    const next = future.current.pop();
    past.current.push(last.current);
    restoring.current = true;
    last.current = next;
    setValue(source, next, { shouldDirty: true });
    bump();
  }, [setValue, source]);

  return {
    undo,
    redo,
    canUndo: past.current.length > 0,
    canRedo: future.current.length > 0,
  };
}
