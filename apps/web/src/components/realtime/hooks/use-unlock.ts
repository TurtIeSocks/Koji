import { useCallback, useContext, useState } from "react";
import { DataProviderContext, useGetIdentity } from "shadmin-core";
import type { Identifier } from "shadmin-core";
import type { Lock, UnlockParams, RealtimeDataProvider } from "../types";

export function useUnlock<R extends string = string>(): {
  unlock: (
    resource: R,
    params: Omit<UnlockParams, "identity"> & { identity?: Identifier },
  ) => Promise<Lock>;
  isLoading: boolean;
  error: Error | null;
} {
  const dataProvider = useContext(
    DataProviderContext,
  ) as RealtimeDataProvider<R> | null;
  if (!dataProvider) {
    throw new Error(
      "useUnlock: no DataProvider found. Must be used inside an Admin or CoreAdminContext.",
    );
  }

  const { identity: defaultIdentity, isLoading: identityLoading } =
    useGetIdentity();
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  const unlock = useCallback(
    async (
      resource: R,
      params: Omit<UnlockParams, "identity"> & { identity?: Identifier },
    ): Promise<Lock> => {
      const identity = params.identity ?? defaultIdentity?.id;
      if (identity == null) {
        throw new Error(
          "useUnlock: no identity available. Pass identity explicitly or configure an authProvider.",
        );
      }
      setIsLoading(true);
      setError(null);
      try {
        const result = await dataProvider.unlock(resource, {
          ...params,
          identity,
        } as UnlockParams);
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

  return { unlock, isLoading: isLoading || identityLoading, error };
}
