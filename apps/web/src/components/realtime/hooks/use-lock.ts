import { useCallback, useContext, useState } from "react";
import { DataProviderContext, useGetIdentity } from "shadmin-core";
import type { Identifier } from "shadmin-core";
import type { Lock, LockParams, RealtimeDataProvider } from "../types";

export function useLock<R extends string = string>(): {
  lock: (
    resource: R,
    params: Omit<LockParams, "identity"> & { identity?: Identifier },
  ) => Promise<Lock>;
  isLoading: boolean;
  error: Error | null;
} {
  const dataProvider = useContext(
    DataProviderContext,
  ) as RealtimeDataProvider<R> | null;
  if (!dataProvider) {
    throw new Error(
      "useLock: no DataProvider found. Must be used inside an Admin or CoreAdminContext.",
    );
  }

  const { identity: defaultIdentity, isLoading: identityLoading } =
    useGetIdentity();
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  const lock = useCallback(
    async (
      resource: R,
      params: Omit<LockParams, "identity"> & { identity?: Identifier },
    ): Promise<Lock> => {
      const identity = params.identity ?? defaultIdentity?.id;
      if (identity == null) {
        throw new Error(
          "useLock: no identity available. Pass identity explicitly or configure an authProvider.",
        );
      }
      setIsLoading(true);
      setError(null);
      try {
        const result = await dataProvider.lock(resource, {
          ...params,
          identity,
        } as LockParams);
        return result;
      } catch (e) {
        const err = e instanceof Error ? e : new Error(String(e));
        setError(err);
        throw err;
      } finally {
        setIsLoading(false);
      }
    },
    [dataProvider, defaultIdentity?.id],
  );

  return { lock, isLoading: isLoading || identityLoading, error };
}
