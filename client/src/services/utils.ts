import type { Map } from 'leaflet'
import { capitalize } from '@mui/material'
import type { FeatureCollection } from 'geojson'
import { UseStore } from '@hooks/useStore'
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

export function rdmToGeojson(
  instance: UseStore['instance'],
  instances: UseStatic['instances'],
): FeatureCollection {
  const geojson: FeatureCollection = {
    type: 'FeatureCollection',
    features: [],
  }
  instance.forEach((ins) => {
    const full = instances[ins]
    if (full) {
      switch (full.type_) {
        case 'auto_quest':
        case 'pokemon_iv':
          {
            const { area }: { area: { lat: number; lon: number }[][] } =
              JSON.parse(full.data)
            if (area) {
              area.forEach((poly) => {
                geojson.features.push({
                  type: 'Feature',
                  properties: {
                    name: ins,
                    type: 'auto_quest',
                  },
                  geometry: {
                    type: 'Polygon',
                    coordinates: [poly.map((p) => [p.lon, p.lat])],
                  },
                })
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
              area.forEach((point) => {
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
          break
        default:
          // eslint-disable-next-line no-console
          console.log('Unknown type:', full.type_)
      }
    }
  })
  return geojson
}

export function rdmToShapes(
  instance: UseStore['instance'],
  instances: UseStatic['instances'],
): Shape[] {
  const shapes: Shape[] = []
  instance.forEach((ins) => {
    const full = instances[ins]
    if (full) {
      switch (full.type_) {
        case 'auto_quest':
        case 'pokemon_iv':
          {
            const { area }: { area: { lat: number; lon: number }[][] } =
              JSON.parse(full.data)
            if (area) {
              area.forEach((poly, i) => {
                shapes.push({
                  id: `${ins}-${i}`,
                  type: 'polygon',
                  positions: poly.map((p) => [p.lat, p.lon]),
                })
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
                shapes.push({
                  id: `${ins}-${i}`,
                  lat: point.lat,
                  lng: point.lon,
                  radius: 10,
                  type: 'circle',
                })
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
              shapes.push({
                lat: area.lat,
                lng: area.lon,
                radius,
                id: ins,
                type: 'circle',
              })
            }
          }
          break
        default:
          // eslint-disable-next-line no-console
          console.log('Unknown type:', full.type_)
      }
    }
  })
  return shapes
}
