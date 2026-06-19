import { useCallback, useContext } from "react";
import { DataProviderContext } from "shadmin-core";
import type {
  RealtimeDataProvider,
  SubscriptionCallback,
  Unsubscribe,
} from "../types";

export function useSubscribeCallback(): <P = unknown>(
  topic: string,
  callback: SubscriptionCallback<P>,
) => Unsubscribe {
  const dataProvider = useContext(
    DataProviderContext,
  ) as RealtimeDataProvider | null;

  return useCallback(
    <P = unknown>(
      topic: string,
      callback: SubscriptionCallback<P>,
    ): Unsubscribe => {
      if (!dataProvider || typeof dataProvider.subscribe !== "function") {
        throw new Error(
          "useSubscribeCallback: dataProvider does not implement subscribe(). Use realtimeDataProvider() to wrap your data provider.",
        );
      }
      return dataProvider.subscribe<P>(topic, callback);
    },
    [dataProvider],
  );
}
