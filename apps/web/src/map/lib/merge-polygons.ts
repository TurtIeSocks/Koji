import { union } from "@turf/union";
import { featureCollection } from "@turf/helpers";

export function mergeSelected(
  fc: GeoJSON.FeatureCollection,
  indexes: number[],
): GeoJSON.FeatureCollection {
  if (indexes.length < 2) return fc;
  const selected = indexes.map((i) => fc.features[i]).filter(Boolean);
  const merged = union(featureCollection(selected as never));
  if (!merged) return fc;
  const rest = fc.features.filter((_, i) => !indexes.includes(i));
  return { type: "FeatureCollection", features: [...rest, merged] };
}
