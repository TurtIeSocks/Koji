import { useContext } from "react";
import { DataProviderContext } from "shadmin-core";
import { useQuery } from "@tanstack/react-query";
import type { Lock, RealtimeDataProvider } from "../types";

export function useGetLocks(resource: string): {
  data: Lock[] | undefined;
  isLoading: boolean;
  isPending: boolean;
  error: Error | null;
} {
  const dataProvider = useContext(
    DataProviderContext,
  ) as RealtimeDataProvider | null;
  if (!dataProvider) {
    throw new Error(
      "useGetLocks: no DataProvider found. Must be used inside an Admin or CoreAdminContext.",
    );
  }

  const { data, isLoading, isPending, error } = useQuery({
    queryKey: [resource, "locks"],
    queryFn: () => dataProvider.getLocks(resource),
  });

  return { data, isLoading, isPending, error: error as Error | null };
}
