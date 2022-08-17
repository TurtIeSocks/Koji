import type { Data } from '@assets/types'
import type { UseStore } from '@hooks/useStore'
import type { Map } from 'leaflet'

import { getMapBounds } from './utils'

export async function getData<T>(
  url: string,
  settings: Partial<UseStore> = {},
): Promise<T> {
  try {
    const data = Object.keys(settings).length
      ? await fetch(url, {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
          },
          body: JSON.stringify({
            instance: settings.instance || '',
            radius: settings.radius || 0.0,
            generations: settings.generations || 0,
          }),
        })
      : await fetch(url)
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

export async function getMarkers(
  map: Map,
  data: UseStore['data'],
  instance: UseStore['instance'],
): Promise<Data> {
  const [pokestops, gyms, spawnpoints] = await Promise.all(
    ['pokestop', 'gym', 'spawnpoint'].map((category) =>
      fetch(
        `/api/data/${data}/${category}`,
        data === 'all'
          ? undefined
          : {
              method: 'POST',
              headers: {
                'Content-Type': 'application/json',
              },
              body: JSON.stringify(
                data === 'bound' ? getMapBounds(map) : { instance },
              ),
            },
      ).then((res) => res.json()),
    ),
  )
  return { pokestops, gyms, spawnpoints }
}
