import { Map } from 'leaflet'

import { Data, PixiMarker, GeoJSON, Point, Line } from '@assets/types'
import { UseStore } from '@hooks/useStore'

export function getMapBounds(map: Map) {
  const mapBounds = map.getBounds()
  const { lat: min_lat, lng: min_lon } = mapBounds.getSouthWest()
  const { lat: max_lat, lng: max_lon } = mapBounds.getNorthEast()
  return { min_lat, max_lat, min_lon, max_lon }
}

export async function getSpawnpoints(map: Map): Promise<PixiMarker[]> {
  const spawnpoints = await fetch('/spawnpoints', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({
      ...getMapBounds(map),
    }),
  })
  return spawnpoints.json()
}

export async function getMarkers(): Promise<Data> {
  const [pokestops, gyms, spawnpoints] = await Promise.all([
    fetch('/pokestops').then((res) => res.json()),
    fetch('/gyms').then((res) => res.json()),
    fetch('/all_spawnpoints').then((res) => res.json()),
  ])
  return { pokestops, gyms, spawnpoints }
}

export async function getSpecificStops(name = ''): Promise<PixiMarker[]> {
  const markers = await fetch('/specific_pokestops', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({
      name,
      radius: 0.0,
      generations: 0,
    }),
  }).then((res) => res.json())
  return markers
}

export async function getGeojson(
  instanceForm: UseStore['instanceForm'],
): Promise<GeoJSON> {
  const points: [number, number][] = await fetch('/quest_generation', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify(instanceForm || {}),
  }).then((res) => res.json())

  const geoJson: GeoJSON = {
    type: 'FeatureCollection',
    features: [
      ...points.map((point, i) => ({
        type: 'Feature',
        properties: {
          id: i,
        },
        geometry: {
          type: 'Point',
          coordinates: [point[1], point[0]] as [number, number],
        } as Point,
      })),
      {
        type: 'Feature',
        properties: {
          id: 0,
          stroke: '#e6194b',
          distance: '74236',
          vehicle_id: 'vehicle_0',
          tour_idx: '0',
          arrival: '2022-05-29T18:05:04Z',
          'stroke-width': '4',
          departure: '2022-05-29T00:00:10Z',
          shift_idx: '0',
          activities: '430',
        },
        geometry: {
          type: 'LineString',
          coordinates: points.map((point) => [point[1], point[0]]),
        } as Line,
      },
    ],
  }
  return geoJson
}

export async function getData<T>(url: string): Promise<T> {
  try {
    const data = await fetch(url)
    if (!data.ok) {
      throw new Error('Failed to fetch data')
    }
    return await data.json()
  } catch (e) {
    // eslint-disable-next-line no-console
    console.error(e)
    return {} as T
  }
}
