import { useGetList } from "shadmin-core";
import type { GetListParams, RaRecord } from "shadmin-core";
import { useQueryClient } from "@tanstack/react-query";
import { useSubscribeToRecordList } from "./use-subscribe-to-record-list";
import { useOnReconnect } from "./use-on-reconnect";

export function useGetListLive<RecordType extends RaRecord = RaRecord>(
  resource: string,
  params: Partial<GetListParams> = {},
  options?: Parameters<typeof useGetList<RecordType>>[2],
): ReturnType<typeof useGetList<RecordType>> {
  const queryClient = useQueryClient();
  const result = useGetList<RecordType>(resource, params, options);

  const invalidate = () => {
    queryClient.invalidateQueries({
      predicate: (q) => q.queryKey[0] === resource,
    });
  };
  useSubscribeToRecordList(resource, invalidate);
  useOnReconnect(invalidate);

  return result;
}
