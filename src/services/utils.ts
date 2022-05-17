import { Map } from 'leaflet'

import { Data, PixiMarker } from '@assets/types'

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
    fetch('/instances').then((res) => res.json()),
  ])
  return { spawnpoints, gyms, pokestops }
}

export async function getData<T>(url: string): Promise<T> {
  const instances = await fetch(url)
  return instances.json()
}