import type { ReactNode } from "react";
import type { Identifier } from "shadmin-core";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Skeleton } from "@/components/ui/skeleton";
import type { Lock } from "./types";
import { LockConflictError } from "./types";
import { useLockOnMount } from "./hooks/use-lock-on-mount";

export interface LockOnMountProps {
  children: ReactNode;
  resource?: string;
  id?: Identifier;
  identity?: Identifier;
  meta?: Record<string, unknown>;
  loading?: ReactNode;
  lockedBy?: (lock: Lock) => ReactNode;
  onLockError?: (error: Error) => void;
}

export function LockOnMount({
  children,
  resource,
  id,
  identity,
  meta,
  loading,
  lockedBy,
  onLockError,
}: LockOnMountProps): ReactNode {
  const { lock, isLocking, lockError } = useLockOnMount({
    resource,
    id,
    identity,
    meta,
    onLockError,
  });

  if (isLocking) {
    return <>{loading ?? <Skeleton className="h-8 w-full" />}</>;
  }

  if (lockError instanceof LockConflictError) {
    return (
      <>
        {lockedBy ? (
          lockedBy(lockError.existingLock)
        ) : (
          <Alert>
            <AlertDescription>
              Locked by {String(lockError.existingLock.identity)}
            </AlertDescription>
          </Alert>
        )}
      </>
    );
  }

  if (lock) return children;
  return null;
}
