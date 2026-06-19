import L from "leaflet";
import type { ShapeKind, GeomanShape } from "../types";

const TYPE_TO_GEOMAN: Record<ShapeKind, GeomanShape> = {
  Point: "Marker",
  MultiPoint: "Marker",
  LineString: "Line",
  MultiLineString: "Line",
  Polygon: "Polygon",
  MultiPolygon: "Polygon",
  GeometryCollection: "Marker",
};

const geojsonTypeToGeomanShape = (t: ShapeKind): GeomanShape =>
  TYPE_TO_GEOMAN[t];

const layerToGeometry = (layer: L.Layer): GeoJSON.Geometry | null => {
  // L.Circle: approximate as a polygon ring (64 vertices around center).
  // GeoJSON has no Circle type and the default `toGeoJSON()` for L.Circle
  // returns a Point, which would lose the radius information.
  if (layer instanceof L.Circle) {
    const center = layer.getLatLng();
    const radiusM = layer.getRadius();
    return circleToPolygon(center.lat, center.lng, radiusM, 64);
  }
  const gj = (
    layer as L.Layer & { toGeoJSON?: () => GeoJSON.Feature }
  ).toGeoJSON?.();
  if (!gj) return null;
  if (gj.type === "Feature") return gj.geometry ?? null;
  if (gj.type === "FeatureCollection") {
    const features = (gj as unknown as GeoJSON.FeatureCollection).features;
    if (features.length === 0) return null;
    return features[0].geometry;
  }
  return gj as unknown as GeoJSON.Geometry;
};

/**
 * Build a closed Polygon ring approximating a geodesic circle. Uses an
 * equirectangular projection at the circle's latitude — accurate enough for
 * radii up to a few hundred km away from the poles.
 */
const circleToPolygon = (
  lat: number,
  lng: number,
  radiusM: number,
  steps: number,
): GeoJSON.Polygon => {
  const earthRadiusM = 6378137;
  const dLat = (radiusM / earthRadiusM) * (180 / Math.PI);
  const dLng = dLat / Math.cos((lat * Math.PI) / 180);
  const ring: GeoJSON.Position[] = [];
  for (let i = 0; i <= steps; i++) {
    const angle = (i / steps) * 2 * Math.PI;
    ring.push([lng + dLng * Math.cos(angle), lat + dLat * Math.sin(angle)]);
  }
  // Force-close: the last point already equals the first due to i = steps.
  return { type: "Polygon", coordinates: [ring] };
};

const geometryToLatLngs = (
  geom: GeoJSON.Geometry,
):
  | L.LatLngExpression
  | L.LatLngExpression[]
  | L.LatLngExpression[][]
  | L.LatLngExpression[][][]
  | unknown[] => {
  switch (geom.type) {
    case "Point":
      return [geom.coordinates[1], geom.coordinates[0]];
    case "MultiPoint":
      return geom.coordinates.map((c) => [c[1], c[0]] as L.LatLngTuple);
    case "LineString":
      return geom.coordinates.map((c) => [c[1], c[0]] as L.LatLngTuple);
    case "MultiLineString":
      return geom.coordinates.map((line) =>
        line.map((c) => [c[1], c[0]] as L.LatLngTuple),
      );
    case "Polygon":
      return geom.coordinates.map((ring) =>
        ring.map((c) => [c[1], c[0]] as L.LatLngTuple),
      );
    case "MultiPolygon":
      return geom.coordinates.map((poly) =>
        poly.map((ring) => ring.map((c) => [c[1], c[0]] as L.LatLngTuple)),
      );
    case "GeometryCollection":
      // Lossy: returns a flat-ish array of latlng nodes from each sub-geometry.
      // Callers using this for rendering should branch on GeometryCollection
      // explicitly instead of relying on a single nested shape.
      return geom.geometries.flatMap((g) => geometryToLatLngs(g) as unknown);
    default:
      return [];
  }
};

const geometryToLayer = (
  geom: GeoJSON.Geometry,
  pathOptions?: L.PathOptions,
  markerIcon?: L.Icon | L.DivIcon,
): L.Layer => {
  return L.geoJSON(geom, {
    style: () => pathOptions ?? {},
    pointToLayer: (_f, latlng) =>
      markerIcon ? L.marker(latlng, { icon: markerIcon }) : L.marker(latlng),
  });
};

export {
  geojsonTypeToGeomanShape,
  layerToGeometry,
  geometryToLatLngs,
  geometryToLayer,
};
