import { useContext, useEffect, useState } from "react";
import { DataProviderContext } from "shadmin-core";
import type {
  RealtimeConnectionStatus,
  RealtimeDataProvider,
  RealtimeTransportError,
} from "../types";

interface RealtimeStatusResult {
  status: RealtimeConnectionStatus;
  lastError: RealtimeTransportError | null;
}

export function useRealtimeStatus(): RealtimeStatusResult {
  const dataProvider = useContext(
    DataProviderContext,
  ) as RealtimeDataProvider | null;
  const hasStatusChange = typeof dataProvider?.onStatusChange === "function";

  const [status, setStatus] = useState<RealtimeConnectionStatus>(
    hasStatusChange ? "disconnected" : "idle",
  );
  const [lastError] = useState<RealtimeTransportError | null>(null);

  useEffect(() => {
    if (!dataProvider || typeof dataProvider.onStatusChange !== "function") {
      return;
    }

    const unsubscribe = dataProvider.onStatusChange((newStatus) => {
      setStatus(newStatus);
    });

    return unsubscribe;
  }, [dataProvider]);

  return { status, lastError };
}
