import { useContext, useEffect, useRef } from "react";
import { DataProviderContext } from "shadmin-core";
import type { RealtimeDataProvider, SubscriptionCallback } from "../types";

export function useSubscribe<P = unknown>(
  topic: string,
  callback: SubscriptionCallback<P>,
  options?: { enabled?: boolean },
): void {
  const dataProvider = useContext(
    DataProviderContext,
  ) as RealtimeDataProvider | null;
  const enabled = options?.enabled !== false;
  const callbackRef = useRef(callback);

  useEffect(() => {
    callbackRef.current = callback;
  });

  useEffect(() => {
    if (!enabled) return;

    if (!dataProvider || typeof dataProvider.subscribe !== "function") {
      throw new Error(
        "useSubscribe: dataProvider does not implement subscribe(). Use realtimeDataProvider() to wrap your data provider.",
      );
    }

    const unsubscribe = dataProvider.subscribe<P>(topic, (event) =>
      callbackRef.current(event),
    );

    return unsubscribe;
  }, [dataProvider, topic, enabled]);
}
