import useDeepCompareEffect from 'use-deep-compare-effect'
import { useShapes } from '@hooks/useShapes'
import type { FeatureCollection } from 'geojson'
import { useStatic } from './useStatic'

export default function useSyncGeojson() {
  const points = useShapes((s) => s.points)
  const multiPoints = useShapes((s) => s.multiPoints)
  const lineStrings = useShapes((s) => s.lineStrings)
  const multiLineStrings = useShapes((s) => s.multiLineStrings)
  const polygons = useShapes((s) => s.polygons)
  const multiPolygons = useShapes((s) => s.multiPolygons)
  const test = useShapes((s) => s.test)

  const geojson = useStatic((s) => s.geojson)
  const setStatic = useStatic((s) => s.setStatic)

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
    test,
  ])

  return geojson
}
