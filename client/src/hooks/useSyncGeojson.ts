import useDeepCompareEffect from 'use-deep-compare-effect'
import { useShapes } from '@hooks/useShapes'
import type { FeatureCollection } from 'geojson'
import { useStatic } from './useStatic'

export default function useSyncGeojson() {
  const points = useShapes((s) => s.Point)
  const multiPoints = useShapes((s) => s.MultiPoint)
  const lineStrings = useShapes((s) => s.LineString)
  const multiLineStrings = useShapes((s) => s.MultiLineString)
  const polygons = useShapes((s) => s.Polygon)
  const multiPolygons = useShapes((s) => s.MultiPolygon)

  const geojson = useStatic((s) => s.geojson)
  const setStatic = useStatic((s) => s.setStatic)

  // eslint-disable-next-line no-console
  console.log('Shape Debug:', useShapes.getState())
  useDeepCompareEffect(() => {
    const newGeojson: FeatureCollection = {
      type: 'FeatureCollection',
      features: [],
    }
    Object.values(points).forEach((point) => newGeojson.features.push(point))
    Object.values(multiPoints).forEach((multiPoint) =>
      newGeojson.features.push(multiPoint),
    )
    Object.values(lineStrings).forEach((lineString) =>
      newGeojson.features.push(lineString),
    )
    Object.values(multiLineStrings).forEach((multiLineString) =>
      newGeojson.features.push(multiLineString),
    )
    Object.values(polygons).forEach((polygon) =>
      newGeojson.features.push(polygon),
    )
    Object.values(multiPolygons).forEach((multiPolygon) =>
      newGeojson.features.push(multiPolygon),
    )
    setStatic('geojson', newGeojson)
  }, [
    points,
    multiPoints,
    lineStrings,
    multiLineStrings,
    polygons,
    multiPolygons,
  ])

  return geojson
}
