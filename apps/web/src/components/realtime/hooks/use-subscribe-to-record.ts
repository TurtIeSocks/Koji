import type { Identifier } from "shadmin-core";
import { recordTopic } from "../topics";
import type { SubscriptionCallback } from "../types";
import { useSubscribe } from "./use-subscribe";

export function useSubscribeToRecord<P = unknown>(
  resource: string,
  id: Identifier | undefined,
  callback: SubscriptionCallback<P>,
  options?: { enabled?: boolean },
): void {
  const enabled = id != null && options?.enabled !== false;
  useSubscribe<P>(id != null ? recordTopic(resource, id) : "", callback, {
    enabled,
  });
}
