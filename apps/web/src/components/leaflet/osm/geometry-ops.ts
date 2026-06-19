import difference from "@turf/difference";
import union from "@turf/union";
import bbox from "@turf/bbox";
import area from "@turf/area";
import { feature, featureCollection } from "@turf/helpers";

const subtract = (
  input: GeoJSON.Polygon | GeoJSON.MultiPolygon,
  mask: GeoJSON.FeatureCollection<GeoJSON.Polygon | GeoJSON.MultiPolygon>,
): GeoJSON.Polygon | GeoJSON.MultiPolygon | null => {
  if (mask.features.length === 0) return input;
  const merged = unionAll(mask as GeoJSON.FeatureCollection<GeoJSON.Polygon>);
  if (!merged) return input;
  const result = difference(
    featureCollection([feature(input), feature(merged)]),
  );
  return result?.geometry ?? null;
};

const unionAll = (
  fc: GeoJSON.FeatureCollection<GeoJSON.Polygon>,
): GeoJSON.Polygon | GeoJSON.MultiPolygon | null => {
  if (fc.features.length === 0) return null;
  if (fc.features.length === 1) return fc.features[0].geometry;
  const result = union(fc);
  return result?.geometry ?? null;
};

const bboxOf = (geom: GeoJSON.Geometry): GeoJSON.BBox => {
  const b = bbox(feature(geom));
  return [b[0], b[1], b[2], b[3]];
};

const areaM2 = (geom: GeoJSON.Polygon | GeoJSON.MultiPolygon): number =>
  area(feature(geom));

const polygonToBBox = (geom: GeoJSON.Geometry): GeoJSON.BBox | null => {
  if (geom.type !== "Polygon") return null;
  return bboxOf(geom);
};

const bboxToPolygon = (bb: unknown): GeoJSON.Polygon | null => {
  if (
    !Array.isArray(bb) ||
    bb.length !== 4 ||
    !bb.every((n) => typeof n === "number")
  ) {
    return null;
  }
  const [w, s, e, n] = bb as GeoJSON.BBox;
  return {
    type: "Polygon",
    coordinates: [
      [
        [w, s],
        [e, s],
        [e, n],
        [w, n],
        [w, s],
      ],
    ],
  };
};

/**
 * Build a value-transform that locks a Polygon's bounding box to a given
 * width/height aspect ratio, centred on the original polygon's bbox centre.
 */
const aspectLockedBBox =
  (ratio: number): ((geom: GeoJSON.Geometry) => GeoJSON.BBox | null) =>
  (geom) => {
    const bb = polygonToBBox(geom);
    if (!bb) return null;
    const [w, s, e, n] = bb;
    const width = e - w;
    const height = n - s;
    if (width === 0 || height === 0) return bb;
    const currentRatio = width / height;
    if (Math.abs(currentRatio - ratio) < 1e-9) return bb;
    const centerX = (w + e) / 2;
    const centerY = (s + n) / 2;
    // Lock by choosing the larger dimension and computing the other from the ratio
    const newWidth = Math.max(width, height * ratio);
    const newHeight = newWidth / ratio;
    return [
      centerX - newWidth / 2,
      centerY - newHeight / 2,
      centerX + newWidth / 2,
      centerY + newHeight / 2,
    ];
  };

export {
  subtract,
  unionAll,
  bboxOf,
  areaM2,
  polygonToBBox,
  bboxToPolygon,
  aspectLockedBBox,
};
