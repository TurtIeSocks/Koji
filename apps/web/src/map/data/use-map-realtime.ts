import { useQueryClient } from "@tanstack/react-query";
import { useSubscribe } from "@/components/realtime";

/** Subscribe to geofence/route realtime events and refetch the map's GeoJSON layers.
 *  No store writes — drives cache invalidation only. */
export function useMapRealtime(): void {
  const qc = useQueryClient();
  useSubscribe("resource/geofence", () =>
    qc.invalidateQueries({ queryKey: ["geo", "geofences"] }),
  );
  useSubscribe("resource/route", () =>
    qc.invalidateQueries({ queryKey: ["geo", "routes"] }),
  );
}
