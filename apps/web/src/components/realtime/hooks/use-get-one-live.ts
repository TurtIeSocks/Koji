import { useGetOne } from "shadmin-core";
import type { GetOneParams, RaRecord } from "shadmin-core";
import { useQueryClient } from "@tanstack/react-query";
import { useSubscribeToRecord } from "./use-subscribe-to-record";
import { useOnReconnect } from "./use-on-reconnect";

export function useGetOneLive<RecordType extends RaRecord = RaRecord>(
  resource: string,
  params: GetOneParams<RecordType>,
  options?: Parameters<typeof useGetOne<RecordType>>[2],
): ReturnType<typeof useGetOne<RecordType>> {
  const queryClient = useQueryClient();
  const result = useGetOne<RecordType>(resource, params, options);

  const invalidate = () => {
    queryClient.invalidateQueries({
      predicate: (q) => q.queryKey[0] === resource,
    });
  };
  useSubscribeToRecord(resource, params.id, invalidate);
  useOnReconnect(invalidate);

  return result;
}
