import type { Map } from 'leaflet'
import { capitalize } from '@mui/material'
import type { FeatureCollection } from 'geojson'

export function getMapBounds(map: Map) {
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
      `${feat.properties?.name}_${feat.properties?.type}`,
      feat,
    ]),
  )
}
