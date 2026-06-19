import { useContext, useEffect } from "react";
import { useGetMany, DataProviderContext } from "shadmin-core";
import type { GetManyParams, RaRecord } from "shadmin-core";
import { useQueryClient } from "@tanstack/react-query";
import { recordTopic } from "../topics";
import type { RealtimeDataProvider } from "../types";
import { useOnReconnect } from "./use-on-reconnect";

export function useGetManyLive<RecordType extends RaRecord = RaRecord>(
  resource: string,
  params: GetManyParams,
  options?: Parameters<typeof useGetMany<RecordType>>[2],
): ReturnType<typeof useGetMany<RecordType>> {
  const queryClient = useQueryClient();
  const dataProvider = useContext(
    DataProviderContext,
  ) as RealtimeDataProvider | null;
  const result = useGetMany<RecordType>(resource, params, options);

  const idsKey = JSON.stringify(params.ids);

  // biome-ignore lint/correctness/useExhaustiveDependencies: queryClient is stable and params.ids is keyed via the serialized idsKey; depending on the raw params.ids array would re-subscribe on every render
  useEffect(() => {
    if (!dataProvider || typeof dataProvider.subscribe !== "function") return;
    const unsubs = params.ids.map((id) =>
      dataProvider.subscribe(recordTopic(resource, id), () => {
        queryClient.invalidateQueries({
          predicate: (q) => q.queryKey[0] === resource,
        });
      }),
    );
    return () => unsubs.forEach((u) => u());
  }, [dataProvider, resource, idsKey]);

  useOnReconnect(() => {
    queryClient.invalidateQueries({
      predicate: (q) => q.queryKey[0] === resource,
    });
  });

  return result;
}
