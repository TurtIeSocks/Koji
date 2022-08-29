import type { Map } from 'leaflet'
import { capitalize } from '@mui/material'
import type { Feature, FeatureCollection, MultiPolygon } from 'geojson'
import { UseStatic } from '@hooks/useStatic'
import { Shape } from '@assets/types'

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
      }
    })
  }
  return flat
}

export function toMultiPolygon(
  area: { lat: number; lon: number }[][],
): MultiPolygon {
  return {
    type: 'MultiPolygon',
    coordinates: [area.map((subArea) => subArea.map((p) => [p.lon, p.lat]))],
  }
}

export function rdmToGeojson(
  selected: UseStatic['selected'],
  instances: UseStatic['instances'],
  existingGeojson: UseStatic['geojson'],
  onlyShapes: true,
): Shape[]

export function rdmToGeojson(
  selected: UseStatic['selected'],
  instances: UseStatic['instances'],
  existingGeojson: UseStatic['geojson'],
  onlyShapes: false,
): FeatureCollection

export function rdmToGeojson(
  selected: UseStatic['selected'],
  instances: UseStatic['instances'],
  existingGeojson: UseStatic['geojson'],
  onlyShapes: boolean,
): FeatureCollection | Shape[] {
  const geojson: FeatureCollection = {
    type: 'FeatureCollection',
    features: existingGeojson.features.filter(
      (x) => x.properties && !x.properties.name,
    ),
  }
  const shapes: Shape[] = []

  selected.forEach((ins) => {
    const full = instances[ins]
    if (full) {
      switch (full.type_) {
        case 'auto_quest':
        case 'pokemon_iv':
          {
            const { area }: { area: { lat: number; lon: number }[][] } =
              JSON.parse(full.data)
            if (area) {
              geojson.features.push({
                type: 'Feature',
                properties: {
                  id: ins,
                  name: full.name,
                  type: full.type_,
                },
                geometry: toMultiPolygon(area),
              })
              area.forEach((poly, i) => {
                if (onlyShapes) {
                  shapes.push({
                    id: `${ins}-${i}`,
                    type: 'polygon',
                    positions: poly.map((p) => [p.lat, p.lon]),
                  })
                }
              })
            }
          }
          break
        case 'circle_pokemon':
        case 'circle_smart_pokemon':
        case 'circle_raid':
        case 'circle_smart_raid':
          {
            const { area }: { area: { lat: number; lon: number }[] } =
              JSON.parse(full.data)
            if (area) {
              area.forEach((point, i) => {
                if (onlyShapes) {
                  shapes.push({
                    id: `${ins}-${i}`,
                    lat: point.lat,
                    lng: point.lon,
                    radius: 10,
                    type: 'circle',
                  })
                } else {
                  geojson.features.push({
                    type: 'Feature',
                    properties: {
                      name: ins,
                      type: 'circle',
                    },
                    geometry: {
                      type: 'Point',
                      coordinates: [point.lon, point.lat],
                    },
                  })
                }
              })
            }
          }
          break
        case 'leveling':
          {
            const {
              area,
              radius,
            }: { area: { lat: number; lon: number }; radius: number } =
              JSON.parse(full.data)
            if (area) {
              if (onlyShapes) {
                shapes.push({
                  lat: area.lat,
                  lng: area.lon,
                  radius,
                  id: ins,
                  type: 'circle',
                })
              } else {
                geojson.features.push({
                  type: 'Feature',
                  properties: {
                    name: ins,
                    type: 'leveling',
                    radius: radius || 10,
                  },
                  geometry: {
                    type: 'Point',
                    coordinates: [area.lon, area.lat],
                  },
                })
              }
            }
          }
          break
        default:
          // eslint-disable-next-line no-console
          console.log('Unknown type:', full.type_)
      }
    }
  })
  return onlyShapes ? shapes : geojson
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
