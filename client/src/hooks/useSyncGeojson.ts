/* eslint-disable no-console */
import useDeepCompareEffect from 'use-deep-compare-effect'

import { FeatureCollection } from '@assets/types'
import { UseShapes, useShapes } from '@hooks/useShapes'
import { s2Coverage } from '@services/fetches'

import { useStatic } from './useStatic'
import { useDbCache } from './useDbCache'

export default function useSyncGeojson() {
  const points = useShapes((s) => s.Point)
  const multiPoints = useShapes((s) => s.MultiPoint)
  const lineStrings = useShapes((s) => s.LineString)
  const multiLineStrings = useShapes((s) => s.MultiLineString)
  const polygons = useShapes((s) => s.Polygon)
  const multiPolygons = useShapes((s) => s.MultiPolygon)

  const geojson = useStatic((s) => s.geojson)

  const setStatic = useStatic((s) => s.setStatic)

  if (process.env.NODE_ENV === 'development') {
    console.log('Shape Debug:', useShapes.getState())
    console.log('Cache Debug:', useDbCache.getState())
  }
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
    Promise.all(
      Object.values(points).map((point) =>
        s2Coverage(
          `${point.properties.__multipoint_id}__${point.id}`,
          point.geometry.coordinates[1],
          point.geometry.coordinates[0],
        ),
      ),
    ).then((results) => {
      const s2cellCoverage: UseShapes['s2cellCoverage'] = {}
      results.forEach((result) => {
        Object.entries(result).forEach(([key, value]) => {
          if (s2cellCoverage[key]) {
            value.forEach((v) => {
              if (!s2cellCoverage[key].includes(v)) {
                s2cellCoverage[key].push(v)
              }
            })
          } else {
            s2cellCoverage[key] = value
          }
        })
      })
      useShapes.setState({
        s2cellCoverage: Object.fromEntries(
          Object.entries(s2cellCoverage).filter(([, v]) => v.length > 0),
        ),
      })
    })
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
