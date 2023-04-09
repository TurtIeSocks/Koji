import intersect from '@turf/intersect'
import pointInPolygon from '@turf/boolean-point-in-polygon'

import { useShapes } from '@hooks/useShapes'
import type { Feature } from '@assets/types'
import type {
  Point,
  MultiPolygon,
  Polygon,
  MultiPoint,
  Position,
} from 'geojson'

export function filterPoints(
  feature: Feature<Polygon | MultiPolygon>,
  contains = false,
  merge = false,
) {
  const {
    Point,
    MultiPoint,
    newRouteCount,
    setters: { remove, add, activeRoute },
  } = useShapes.getState()
  const merged: Feature<MultiPoint> = {
    type: 'Feature',
    id: `newRoute${newRouteCount + 1}`,
    geometry: {
      type: 'MultiPoint',
      coordinates: [],
    },
    properties: {},
  }

  Object.entries(Point).forEach(([key, value]) => {
    const inside = pointInPolygon(value, feature.geometry)
    if (contains ? inside : !inside) {
      remove(value.geometry.type, key)
      if (merge) {
        merged.geometry.coordinates.push(value.geometry.coordinates)
      }
    }
  })
  Object.entries(MultiPoint).forEach(([key, value]) => {
    const points: Point[] = value.geometry.coordinates.map((c) => ({
      type: 'Point',
      coordinates: c,
    }))
    const others: Position[] = []
    const filtered = points.filter((p) => {
      const inside = pointInPolygon(p, feature.geometry)
      if (merge) {
        if (contains ? !inside : inside) {
          others.push(p.coordinates)
        }
      }
      return contains ? inside : !inside
    })
    if (filtered.length) {
      remove(value.geometry.type, key)
      if (merge) {
        merged.geometry.coordinates.push(...filtered.map((p) => p.coordinates))
      }
      if (filtered.length !== points.length) {
        if (merge) {
          add({
            ...value,
            geometry: {
              type: 'MultiPoint',
              coordinates: others,
            },
          })
        } else {
          add({
            ...value,
            geometry: {
              type: 'MultiPoint',
              coordinates: filtered.map((p) => p.coordinates),
            },
          })
        }
      }
    }
  })
  if (merged.geometry.coordinates.length) {
    useShapes.setState({ newRouteCount: newRouteCount + 1 })
    add(merged)
    activeRoute()
  }
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
