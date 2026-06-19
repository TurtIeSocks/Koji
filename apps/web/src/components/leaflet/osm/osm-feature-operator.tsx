"use client";

import { useEffect, useState } from "react";
import { useFormContext, useWatch } from "react-hook-form";
import { useNotify, useTranslate } from "shadmin-core";
import { Loader2 } from "lucide-react";

import { Button } from "@/components/ui/button";

import { useOsmFeatures } from "./use-osm-features";
import { type OsmPresetName } from "./osm-presets";
import type { OsmTagInput } from "./osm-tag-catalog";
import { bboxOf, subtract, unionAll, areaM2 } from "./geometry-ops";

type OsmFeatureMode = "subtract" | "add";

interface OsmFeatureOperatorProps {
  source: string;
  presets?: ReadonlyArray<OsmPresetName>;
  tags?: ReadonlyArray<OsmTagInput>;
  mode: OsmFeatureMode;
  label?: string;
  icon?: React.ComponentType<{ className?: string }>;
  endpoint?: string;
}

const OsmFeatureOperator = ({
  source,
  presets,
  tags,
  mode,
  label,
  icon: Icon,
  endpoint,
}: OsmFeatureOperatorProps) => {
  const form = useFormContext();
  const translate = useTranslate();
  const notify = useNotify();
  const value = useWatch({ name: source }) as
    | GeoJSON.Polygon
    | GeoJSON.MultiPolygon
    | null;
  const [bbox, setBbox] = useState<GeoJSON.BBox | null>(null);
  const osm = useOsmFeatures(bbox, { presets, tags }, { endpoint });

  const hasSources = (presets?.length ?? 0) + (tags?.length ?? 0) > 0;

  const handleClick = () => {
    if (!hasSources) {
      notify("No OSM presets or tags configured", { type: "warning" });
      return;
    }
    if (!value) {
      notify("No polygon to operate on", { type: "warning" });
      return;
    }
    setBbox(bboxOf(value));
  };

  useEffect(() => {
    if (!osm.data || !bbox || !value) return;
    const mask = osm.data;
    const polygonMask = {
      ...mask,
      features: mask.features.filter(
        (f): f is GeoJSON.Feature<GeoJSON.Polygon> =>
          f.geometry?.type === "Polygon" || f.geometry?.type === "MultiPolygon",
      ),
    } as GeoJSON.FeatureCollection<GeoJSON.Polygon>;

    if (mode === "subtract") {
      if (polygonMask.features.length === 0) {
        notify("No matching features in bbox", { type: "info" });
        setBbox(null);
        return;
      }
      const result = subtract(value, polygonMask);
      if (result === null) {
        form.setValue(source, null, { shouldDirty: true });
        notify("Polygon entirely covered; cleared", { type: "warning" });
      } else {
        const removedArea = areaM2(value) - areaM2(result);
        form.setValue(source, result, { shouldDirty: true });
        notify(
          translate("ra.leaflet.osm.subtracted", {
            _: `Subtracted ${formatKm2(removedArea)} km² from polygon`,
            area: formatKm2(removedArea),
          }),
          { type: "success" },
        );
      }
    } else {
      // mode === "add": union the source polygon with all matching mask polygons
      if (polygonMask.features.length === 0) {
        notify("No matching features in bbox to add", { type: "info" });
        setBbox(null);
        return;
      }
      const combined: GeoJSON.FeatureCollection<GeoJSON.Polygon> = {
        type: "FeatureCollection",
        features: [
          {
            type: "Feature",
            geometry: value as GeoJSON.Polygon,
            properties: {},
          },
          ...polygonMask.features,
        ],
      };
      const result = unionAll(combined);
      if (!result) {
        notify("Union produced no geometry", { type: "warning" });
      } else {
        const addedArea = areaM2(result) - areaM2(value);
        form.setValue(source, result, { shouldDirty: true });
        notify(
          translate("ra.leaflet.osm.added", {
            _: `Added ${formatKm2(addedArea)} km² to polygon`,
            area: formatKm2(addedArea),
          }),
          { type: "success" },
        );
      }
    }
    setBbox(null);
  }, [osm.data, bbox, value, mode, source, form, notify, translate]);

  return (
    <Button
      type="button"
      onClick={handleClick}
      disabled={osm.isLoading}
      variant="outline"
    >
      {osm.isLoading ? (
        <Loader2 className="animate-spin" />
      ) : Icon ? (
        <Icon />
      ) : null}
      {label}
    </Button>
  );
};

const formatKm2 = (m2: number) => (Math.round(m2 / 1000) / 1000).toFixed(3);

export {
  type OsmFeatureMode,
  type OsmFeatureOperatorProps,
  OsmFeatureOperator,
};
