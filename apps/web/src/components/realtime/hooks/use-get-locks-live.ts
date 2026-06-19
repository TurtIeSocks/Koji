import { useQueryClient } from "@tanstack/react-query";
import type { Lock } from "../types";
import { lockResourceTopic } from "../topics";
import { useGetLocks } from "./use-get-locks";
import { useSubscribe } from "./use-subscribe";
import { useOnReconnect } from "./use-on-reconnect";

export function useGetLocksLive(resource: string): {
  data: Lock[] | undefined;
  isLoading: boolean;
  isPending: boolean;
  error: Error | null;
} {
  const queryClient = useQueryClient();
  const result = useGetLocks(resource);

  const invalidate = () => {
    queryClient.invalidateQueries({
      queryKey: [resource, "locks"],
    });
  };

  useSubscribe(lockResourceTopic(resource), invalidate);
  useOnReconnect(invalidate);

  return result;
}
