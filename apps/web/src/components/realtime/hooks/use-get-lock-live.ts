import { useQueryClient } from "@tanstack/react-query";
import type { GetLockParams, Lock } from "../types";
import { lockTopic } from "../topics";
import { useGetLock } from "./use-get-lock";
import { useSubscribe } from "./use-subscribe";
import { useOnReconnect } from "./use-on-reconnect";

export function useGetLockLive(
  resource: string,
  params: GetLockParams,
): {
  data: Lock | null | undefined;
  isLoading: boolean;
  isPending: boolean;
  error: Error | null;
} {
  const queryClient = useQueryClient();
  const result = useGetLock(resource, params);

  const invalidate = () => {
    queryClient.invalidateQueries({
      queryKey: [resource, "lock", params.id],
    });
  };

  useSubscribe(lockTopic(resource, params.id), invalidate);
  useOnReconnect(invalidate);

  return result;
}
