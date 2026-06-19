import { useCallback, useContext } from "react";
import { DataProviderContext } from "shadmin-core";
import type { RealtimeDataProvider, RealtimeEvent } from "../types";

export function usePublish(): <P = unknown>(
  topic: string,
  event: Omit<RealtimeEvent<P>, "topic">,
) => Promise<void> {
  const dataProvider = useContext(
    DataProviderContext,
  ) as RealtimeDataProvider | null;

  return useCallback(
    <P = unknown>(
      topic: string,
      event: Omit<RealtimeEvent<P>, "topic">,
    ): Promise<void> => {
      if (!dataProvider || typeof dataProvider.publish !== "function") {
        throw new Error(
          "usePublish: dataProvider does not implement publish(). Use realtimeDataProvider() to wrap your data provider.",
        );
      }
      return dataProvider.publish<P>(topic, event);
    },
    [dataProvider],
  );
}
