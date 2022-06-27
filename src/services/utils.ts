import { Map } from 'leaflet'
import distance from '@turf/distance'

import { Data, PixiMarker } from '@assets/types'
import { UseStore } from '@hooks/useStore'

export function getMapBounds(map: Map) {
  const mapBounds = map.getBounds()
  const { lat: min_lat, lng: min_lon } = mapBounds.getSouthWest()
  const { lat: max_lat, lng: max_lon } = mapBounds.getNorthEast()
  return { min_lat, max_lat, min_lon, max_lon }
}

export async function getSpawnpoints(map: Map): Promise<PixiMarker[]> {
  const spawnpoints = await fetch('/api/spawnpoints', {
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
    fetch('/api/pokestops').then((res) => res.json()),
    fetch('/api/gyms').then((res) => res.json()),
    fetch('/api/all_spawnpoints').then((res) => res.json()),
  ])
  return { pokestops, gyms, spawnpoints }
}

export async function getSpecificStops(name = ''): Promise<PixiMarker[]> {
  const markers = await fetch('/api/specific_pokestops', {
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
): Promise<[number, number][]> {
  return fetch('/api/quest_generation', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify(instanceForm || {}),
  }).then((res) => res.json())
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

export function getColor(start: [number, number], end: [number, number]) {
  const dis = distance(start, end, { units: 'meters' })
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
