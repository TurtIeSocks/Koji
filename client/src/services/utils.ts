import * as L from 'leaflet'
import { capitalize } from '@mui/material'
import type { Feature, FeatureCollection, MultiPolygon, Polygon } from 'geojson'
import union from '@turf/union'
import bbox from '@turf/bbox'

export function getMapBounds(map: L.Map) {
  const mapBounds = map.getBounds()
  const { lat: min_lat, lng: min_lon } = mapBounds.getSouthWest()
  const { lat: max_lat, lng: max_lon } = mapBounds.getNorthEast()
  return { min_lat, max_lat, min_lon, max_lon }
}

export function getColor(dis: number) {
  switch (true) {
    case dis <= 500:
      return 'green'
    case dis <= 1000:
      return 'yellow'
    case dis <= 1500:
      return 'orange'
    default:
      return 'red'
  }
}

export function fromCamelCase(str: string, separator = ' '): string {
  return capitalize(str)
    .replace(/([a-z\d])([A-Z])/g, `$1${separator}$2`)
    .replace(/([A-Z]+)([A-Z][a-z\d]+)/g, `$1${separator}$2`)
}

export function fromSnakeCase(str: string, separator = ' '): string {
  return capitalize(str)
    .replace(/_/g, separator)
    .replace(/([a-z\d])([A-Z])/g, `$1${separator}$2`)
    .replace(/([A-Z]+)([A-Z][a-z\d]+)/g, `$1${separator}$2`)
}

export function safeParse<T>(value: string): {
  value: T
  error: boolean | string
} {
  try {
    return { value: JSON.parse(value), error: false }
  } catch (e) {
    if (e instanceof Error) {
      return { error: e.message, value: null as T }
    }
    return { error: true, value: null as T }
  }
}

export function collectionToObject(collection: FeatureCollection) {
  return Object.fromEntries(
    collection.features.map((feat) => [
      `${feat.properties?.__name}_${feat.properties?.__type}`,
      feat,
    ]),
  )
}

export function filterImports<T extends Feature>(
  existing: Record<string, T>,
): Record<string, T> {
  return Object.fromEntries(
    Object.values(existing)
      .filter((feat) => typeof feat.id === 'number')
      .map((feat) => [feat.id, feat]),
  )
}

export function combineByProperty(
  featureCollection: FeatureCollection,
  key = 'name',
): FeatureCollection {
  const featureHash: Record<string, Feature<Polygon | MultiPolygon>> = {}
  featureCollection.features.forEach((feat) => {
    const name = feat.properties?.[key]
    if (
      name &&
      (feat.geometry.type === 'Polygon' ||
        feat.geometry.type === 'MultiPolygon')
    ) {
      const existing = featureHash[name]
      if (existing) {
        const merged = union(existing, feat as Feature<Polygon | MultiPolygon>)
        if (merged) {
          featureHash[name] = {
            ...existing,
            ...merged,
            properties: {
              ...existing.properties,
              ...feat.properties,
            },
          }
        }
      } else {
        featureHash[name] = feat as Feature<Polygon | MultiPolygon>
      }
    }
  })
  return {
    ...featureCollection,
    bbox: bbox(featureCollection),
    features: Object.values(featureHash).map((feat) => ({
      ...feat,
      bbox: bbox(feat),
    })),
  }
}

export function splitMultiPolygons(
  featureCollection: FeatureCollection,
): FeatureCollection {
  const features: Feature[] = []
  featureCollection.features.forEach((feature: Feature) => {
    if (feature.geometry.type === 'MultiPolygon') {
      const { coordinates } = feature.geometry
      coordinates.forEach((polygon, i) => {
        features.push({
          ...feature,
          properties: {
            ...feature.properties,
            name:
              coordinates.length === 1
                ? feature.properties?.__name || ''
                : `${feature.properties?.__name}_${i}`,
          },
          geometry: {
            ...feature.geometry,
            type: 'Polygon',
            coordinates: polygon,
          },
        })
      })
    } else {
      features.push(feature)
    }
  })
  return {
    ...featureCollection,
    features,
  }
}
