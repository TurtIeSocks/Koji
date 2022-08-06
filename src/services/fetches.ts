import type { Data, PixiMarker } from '@assets/types'

import { getMapBounds } from './utils'

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

export async function getMarkers(): Promise<Data> {
  const [pokestops, gyms, spawnpoints] = await Promise.all([
    fetch('/api/pokestop/all').then((res) => res.json()),
    fetch('/api/gym/all').then((res) => res.json()),
    fetch('/api/spawnpoint/all').then((res) => res.json()),
  ])
  return { pokestops, gyms, spawnpoints }
}

export async function getSpawnpoints(): Promise<PixiMarker[]> {
  const spawnpoints = await fetch('/api/spawnpoints', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({
      ...getMapBounds(),
    }),
  })
  return spawnpoints.json()
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
