import { resourceTopic } from "../topics";
import type { SubscriptionCallback } from "../types";
import { useSubscribe } from "./use-subscribe";

export function useSubscribeToRecordList<P = unknown>(
  resource: string,
  callback: SubscriptionCallback<P>,
  options?: { enabled?: boolean },
): void {
  useSubscribe<P>(resourceTopic(resource), callback, options);
}
