import type { Map } from 'leaflet'
import { capitalize } from '@mui/material'
import type { Feature, FeatureCollection } from 'geojson'
import { ArrayInput, ObjectInput } from '@assets/types'

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

export function convertGeojson(geojson?: FeatureCollection) {
  const flat: [number, number][][] = []
  if (geojson) {
    geojson.features.forEach((feature) => {
      if (feature.geometry.type === 'Polygon') {
        feature.geometry.coordinates.forEach((coordinates) => {
          const stuff: [number, number][] = coordinates.map(([lng, lat]) => [
            lat,
            lng,
          ])
          flat.push(stuff)
        })
      } else if (feature.geometry.type === 'MultiPolygon') {
        feature.geometry.coordinates.forEach((coordinates) => {
          coordinates.forEach((coord) => {
            const stuff: [number, number][] = coord.map(([lng, lat]) => [
              lat,
              lng,
            ])
            flat.push(stuff)
          })
        })
      }
    })
  }
  return flat
}

export function safeParse(value: string) {
  try {
    return JSON.parse(value)
  } catch (e) {
    if (e instanceof Error) {
      return { error: e.message }
    }
    return { error: true }
  }
}
export function geojsonToExport(
  geojson: Feature,
  object: true,
): { lat: number; lon: number }[][]
export function geojsonToExport(geojson: Feature, object?: false): number[][][]
export function geojsonToExport(
  geojson: Feature,
  object = false,
): number[][][] | { lat: number; lon: number }[][] {
  switch (geojson.geometry.type) {
    case 'Point':
      return object
        ? [
            [
              {
                lat: geojson.geometry.coordinates[1],
                lon: geojson.geometry.coordinates[0],
              },
            ],
          ]
        : [[[geojson.geometry.coordinates[1], geojson.geometry.coordinates[0]]]]
    case 'Polygon':
      return object
        ? geojson.geometry.coordinates.map((poly) =>
            poly.map((p) => ({ lat: p[1], lon: p[0] })),
          )
        : geojson.geometry.coordinates.map((poly) =>
            poly.map((p) => [p[1], p[0]]),
          )
    case 'MultiPolygon':
      return object
        ? geojson.geometry.coordinates.flatMap((poly) =>
            poly.map((p) => p.map((x) => ({ lat: x[1], lon: x[0] }))),
          )
        : geojson.geometry.coordinates.flatMap((poly) =>
            poly.map((p) => p.map((x) => [x[1], x[0]])),
          )
    default:
      return []
  }
}

export function textToFeature(input: string): Feature {
  return {
    type: 'Feature',
    properties: {},
    geometry: {
      type: 'Polygon',
      coordinates: [
        input.split('\n').map((line) => {
          const [lat, lon] = line.split(',').map((x) => +x.trim())
          if (
            !lat ||
            typeof lat !== 'number' ||
            !lon ||
            typeof lon !== 'number'
          ) {
            throw new Error('Invalid input')
          }
          return [lon, lat]
        }),
      ],
    },
  }
}

export function arrayToFeature(input: ObjectInput | ArrayInput): Feature {
  return {
    type: 'Feature',
    properties: {},
    geometry: {
      type: 'Polygon',
      coordinates: input.map((line) =>
        line.map((x) => (Array.isArray(x) ? [x[1], x[0]] : [x.lon, x.lat])),
      ),
    },
  }
}
