import type { ReactElement, ReactNode } from "react";
import {
  type Identifier,
  useRecordContext,
  useResourceContext,
} from "shadmin-core";
import { Badge } from "@/components/ui/badge";
import type { Lock } from "./types";
import { useGetLockLive } from "./hooks/use-get-lock-live";

export interface LockStatusProps {
  resource?: string;
  id?: Identifier;
  renderLock?: (lock: Lock) => ReactNode;
  renderFree?: () => ReactNode;
}

export function LockStatus({
  resource,
  id,
  renderLock,
  renderFree,
}: LockStatusProps): ReactElement | null {
  const ctxResource = useResourceContext();
  const ctxRecord = useRecordContext();
  const effectiveResource = resource ?? ctxResource ?? "";
  const effectiveId = id ?? ctxRecord?.id ?? "";

  // Hooks must run unconditionally — call with sentinel values when missing.
  const { data: lock } = useGetLockLive(effectiveResource, { id: effectiveId });

  if (!effectiveResource || effectiveId === "") return null;

  if (lock) {
    return (
      <>
        {renderLock ? (
          renderLock(lock)
        ) : (
          <Badge variant="secondary">Locked by {String(lock.identity)}</Badge>
        )}
      </>
    );
  }

  if (renderFree) return <>{renderFree()}</>;
  return null;
}
