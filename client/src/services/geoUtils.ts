import intersect from '@turf/intersect'
import pointInPolygon from '@turf/boolean-point-in-polygon'

import { useShapes } from '@hooks/useShapes'
import type { Feature } from '@assets/types'
import type { Point, MultiPolygon, Polygon } from 'geojson'

export function filterPoints(
  feature: Feature<Polygon | MultiPolygon>,
  contains = false,
) {
  const {
    Point,
    MultiPoint,
    setters: { remove, add },
  } = useShapes.getState()
  Object.entries(Point).forEach(([key, value]) => {
    const inside = pointInPolygon(value, feature.geometry)
    if (contains ? inside : !inside) remove(value.geometry.type, key)
  })
  Object.entries(MultiPoint).forEach(([key, value]) => {
    const points: Point[] = value.geometry.coordinates.map((c) => ({
      type: 'Point',
      coordinates: c,
    }))
    const filtered = points.filter((p) => {
      const inside = pointInPolygon(p, feature.geometry)
      return contains ? inside : !inside
    })
    if (filtered.length) {
      remove(value.geometry.type, key)
      if (filtered.length !== points.length) {
        add({
          ...value,
          geometry: {
            type: 'MultiPoint',
            coordinates: filtered.map((p) => p.coordinates),
          },
        })
      }
    }
  })
}

export function filterPolys(
  feature: Feature<Polygon | MultiPolygon>,
  contains = false,
) {
  const {
    Polygon,
    MultiPolygon,
    setters: { remove },
  } = useShapes.getState()
  const all = { ...Polygon, ...MultiPolygon }
  Object.entries(all).forEach(([key, value]) => {
    const inside = intersect(feature, value)
    if (key !== feature.id && (contains ? inside : !inside))
      remove(value.geometry.type, key)
  })
}
