import { useEffect, useRef, useState } from "react";
import {
  useGetIdentity,
  useRecordContext,
  useResourceContext,
} from "shadmin-core";
import type { Identifier } from "shadmin-core";
import type { Lock } from "../types";
import { useLock } from "./use-lock";
import { useUnlock } from "./use-unlock";

export interface UseLockOnMountOptions {
  resource?: string;
  id?: Identifier;
  identity?: Identifier;
  meta?: Record<string, unknown>;
  onLockError?: (error: Error) => void;
}

export function useLockOnMount(options: UseLockOnMountOptions = {}): {
  lock: Lock | null;
  isLocking: boolean;
  lockError: Error | null;
} {
  const ctxResource = useResourceContext();
  const ctxRecord = useRecordContext();
  const { identity: ctxIdentity } = useGetIdentity();
  const { lock } = useLock();
  const { unlock } = useUnlock();
  const heldRef = useRef<Lock | null>(null);

  const [held, setHeld] = useState<Lock | null>(null);
  const [error, setError] = useState<Error | null>(null);
  const [isLocking, setIsLocking] = useState(false);

  const resource = options.resource ?? ctxResource;
  const id = options.id ?? ctxRecord?.id;
  const identity = options.identity ?? ctxIdentity?.id;

  // biome-ignore lint/correctness/useExhaustiveDependencies: only the resource/id/identity triple drives the lock-on-mount; lock/unlock are recreated on identity changes and options is a fresh object each render, so depending on them would re-lock spuriously
  useEffect(() => {
    if (!resource || id == null || identity == null) return;
    let cancelled = false;
    setIsLocking(true);
    lock(resource, { id, identity, meta: options.meta })
      .then((result) => {
        if (cancelled) return;
        heldRef.current = result;
        setHeld(result);
      })
      .catch((e: Error) => {
        if (cancelled) return;
        setError(e);
        options.onLockError?.(e);
      })
      .finally(() => {
        if (!cancelled) setIsLocking(false);
      });

    return () => {
      cancelled = true;
      const heldLock = heldRef.current;
      if (heldLock) {
        unlock(resource, { id, identity }).catch(options.onLockError);
        heldRef.current = null;
      }
    };
  }, [resource, id, identity]);

  return { lock: held, isLocking, lockError: error };
}
