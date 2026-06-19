import { useContext } from "react";
import { DataProviderContext } from "shadmin-core";
import { useQuery } from "@tanstack/react-query";
import type { GetLockParams, Lock, RealtimeDataProvider } from "../types";

export function useGetLock(
  resource: string,
  params: GetLockParams,
): {
  data: Lock | null | undefined;
  isLoading: boolean;
  isPending: boolean;
  error: Error | null;
} {
  const dataProvider = useContext(
    DataProviderContext,
  ) as RealtimeDataProvider | null;
  if (!dataProvider) {
    throw new Error(
      "useGetLock: no DataProvider found. Must be used inside an Admin or CoreAdminContext.",
    );
  }

  const { data, isLoading, isPending, error } = useQuery({
    queryKey: [resource, "lock", params.id],
    queryFn: () => dataProvider.getLock(resource, params),
  });

  return { data, isLoading, isPending, error: error as Error | null };
}
