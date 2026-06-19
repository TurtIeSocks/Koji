import type { DataProvider } from "shadmin-core";
import type {
  GetLockParams,
  GetLocksParams,
  Lock,
  LockParams,
  RealtimeDataProvider,
  RealtimeDataProviderOptions,
  RealtimeTransport,
  SubscriptionCallback,
  UnlockParams,
  Unsubscribe,
} from "./types";

function locksOrReject<R extends string, T>(
  methodName: string,
  options: RealtimeDataProviderOptions<R> | undefined,
  fn: (
    locks: NonNullable<RealtimeDataProviderOptions<R>["locks"]>,
  ) => Promise<T>,
): Promise<T> {
  const locks = options?.locks;
  if (!locks) {
    return Promise.reject(
      new Error(
        `realtimeDataProvider: ${methodName}() called but no LockProvider was configured. Pass { locks } in the third argument to enable locking.`,
      ),
    );
  }
  return fn(locks);
}

export function realtimeDataProvider<R extends string = string>(
  baseDataProvider: DataProvider<R>,
  transport: RealtimeTransport,
  options?: RealtimeDataProviderOptions<R>,
): RealtimeDataProvider<R> {
  const provider: RealtimeDataProvider<R> = {
    ...baseDataProvider,

    subscribe<P = unknown>(
      topic: string,
      cb: SubscriptionCallback<P>,
    ): Unsubscribe {
      return transport.subscribe<P>(topic, cb);
    },

    publish<P = unknown>(
      topic: string,
      event: Parameters<RealtimeTransport["publish"]>[1],
    ): Promise<void> {
      return transport.publish<P>(
        topic,
        event as Parameters<typeof transport.publish<P>>[1],
      );
    },

    lock(resource: R, params: LockParams): Promise<Lock> {
      return locksOrReject("lock", options, (l) => l.lock(resource, params));
    },

    unlock(resource: R, params: UnlockParams): Promise<Lock> {
      return locksOrReject("unlock", options, (l) =>
        l.unlock(resource, params),
      );
    },

    getLock(resource: R, params: GetLockParams): Promise<Lock | null> {
      return locksOrReject("getLock", options, (l) =>
        l.getLock(resource, params),
      );
    },

    getLocks(resource: R, params?: GetLocksParams): Promise<Lock[]> {
      return locksOrReject("getLocks", options, (l) =>
        l.getLocks(resource, params),
      );
    },

    ...(transport.onReconnect
      ? {
          onReconnect: (cb: () => void): Unsubscribe =>
            transport.onReconnect!(cb),
        }
      : {}),

    ...(transport.onStatusChange
      ? {
          onStatusChange: (
            cb: Parameters<NonNullable<RealtimeTransport["onStatusChange"]>>[0],
          ): Unsubscribe => transport.onStatusChange!(cb),
        }
      : {}),
  };

  return provider;
}
