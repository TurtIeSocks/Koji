import { useContext, useEffect, useRef } from "react";
import { DataProviderContext } from "shadmin-core";
import type { RealtimeDataProvider } from "../types";

export function useOnReconnect(callback: () => void): void {
  const dataProvider = useContext(
    DataProviderContext,
  ) as RealtimeDataProvider | null;
  const callbackRef = useRef(callback);

  useEffect(() => {
    callbackRef.current = callback;
  });

  useEffect(() => {
    if (!dataProvider || typeof dataProvider.onReconnect !== "function") {
      return;
    }

    const unsubscribe = dataProvider.onReconnect(() => {
      callbackRef.current();
    });

    return unsubscribe;
  }, [dataProvider]);
}
