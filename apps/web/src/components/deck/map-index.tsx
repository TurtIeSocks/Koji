import { useMemo } from "react";
import { useNavigate } from "react-router";
import { GeoJsonLayer } from "@deck.gl/layers";
import type { Layer, PickingInfo } from "@deck.gl/core";
import { useGeoFeatures } from "@/map/data/use-geo-features";
import { DeckMap } from "./deck-map";
import { geometryBounds } from "./bounds";

/** Read-only overworld: all geofences at once, click one -> its own page. No
 *  editing/calc/draw here (that lives per-entity, see GeofenceMap/RouteMap). */
export function MapIndex() {
  const navigate = useNavigate();
  const { data } = useGeoFeatures("geofences", true);
  const fc = data ?? { type: "FeatureCollection", features: [] };

  const layers = useMemo<Layer[]>(
    () => [
      new GeoJsonLayer({
        id: "geofences",
        data: fc,
        filled: true,
        getFillColor: [255, 140, 0, 40],
        getLineColor: [255, 140, 0, 220],
        lineWidthMinPixels: 1,
        pickable: true,
        onClick: (info: PickingInfo) => {
          const f = info.object as GeoJSON.Feature | undefined;
          const id = f?.id ?? (f?.properties as { id?: string | number } | null)?.id;
          if (id != null) navigate(`/geofence/${id}`);
        },
      }),
    ],
    [fc, navigate],
  );

  const fit = fc.features.length ? geometryBounds(fc) : null;
  return <DeckMap layers={layers} fitBounds={fit} height="100%" controller />;
}
