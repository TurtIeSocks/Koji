import type {
  Lock,
  LockParams,
  UnlockParams,
  GetLockParams,
  LockProvider,
  RealtimeTransport,
} from "../types";
import { LockConflictError } from "../types";
import { lockTopic, lockResourceTopic } from "../topics";

export interface InMemoryLockProviderOptions {
  publisher?: Pick<RealtimeTransport, "publish">;
}

export function inMemoryLockProvider<R extends string = string>(
  options?: InMemoryLockProviderOptions,
): LockProvider<R> {
  const store = new Map<string, Lock>();

  function key(resource: string, id: unknown): string {
    return `${resource}::${String(id)}`;
  }

  async function lock(resource: R, params: LockParams): Promise<Lock> {
    const k = key(resource, params.id);
    const existing = store.get(k);

    if (existing) {
      if (String(existing.identity) === String(params.identity)) {
        return existing;
      }
      throw new LockConflictError(existing);
    }

    const newLock: Lock = {
      resource,
      recordId: params.id,
      identity: params.identity,
      createdAt: new Date().toISOString(),
    };
    store.set(k, newLock);

    await options?.publisher?.publish(lockTopic(resource, params.id), {
      type: "locked",
      payload: newLock,
    });
    // Also broadcast on the collection topic so useGetLocksLive subscribers wake up.
    await options?.publisher?.publish(lockResourceTopic(resource), {
      type: "locked",
      payload: newLock,
    });

    return newLock;
  }

  async function unlock(resource: R, params: UnlockParams): Promise<Lock> {
    const k = key(resource, params.id);
    const existing = store.get(k);

    if (!existing) {
      throw new Error(
        `inMemoryLockProvider: unlock() called but no lock exists for ${resource}/${String(params.id)}`,
      );
    }

    if (String(existing.identity) !== String(params.identity)) {
      throw new LockConflictError(existing);
    }

    store.delete(k);

    await options?.publisher?.publish(lockTopic(resource, params.id), {
      type: "unlocked",
      payload: existing,
    });
    await options?.publisher?.publish(lockResourceTopic(resource), {
      type: "unlocked",
      payload: existing,
    });

    return existing;
  }

  async function getLock(
    resource: R,
    params: GetLockParams,
  ): Promise<Lock | null> {
    return store.get(key(resource, params.id)) ?? null;
  }

  async function getLocks(resource: R): Promise<Lock[]> {
    const result: Lock[] = [];
    for (const lock of store.values()) {
      if (lock.resource === resource) {
        result.push(lock);
      }
    }
    return result;
  }

  return { lock, unlock, getLock, getLocks };
}
