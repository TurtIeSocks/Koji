/* eslint-disable no-console */
/* eslint-disable no-nested-ternary */
import type { Map } from 'leaflet'
import type { Feature, FeatureCollection } from 'geojson'

import type { CombinedState, Data, ToConvert } from '@assets/types'
import type { UsePersist } from '@hooks/usePersist'
import type { UseStatic } from '@hooks/useStatic'

import { fromSnakeCase, getMapBounds } from './utils'

export async function getData<T>(
  url: string,
  settings: CombinedState & { area?: Feature } = {},
): Promise<T | null> {
  try {
    const data = Object.keys(settings).length
      ? await fetch(url, {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
          },
          body: JSON.stringify(settings),
        })
      : await fetch(url)
    const body = await data.json()
    if (!data.ok) {
      throw new Error(body.message)
    }
    return body
  } catch (e) {
    console.error(e)
    return null
  }
}

export async function getLotsOfData(
  url: string,
  setStatic: UseStatic['setStatic'],
  settings: CombinedState = {},
): Promise<FeatureCollection> {
  const { length = 0 } = settings.geojson?.features || {}
  const totalStartTime = Date.now()
  const features = await Promise.allSettled(
    (settings.geojson?.features || [])
      .filter(
        (x) =>
          x.geometry.type === 'Polygon' || x.geometry.type === 'MultiPolygon',
      )
      .map(async (area) => {
        const startTime = Date.now()
        return fetch(url, {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
          },
          body: JSON.stringify({
            ...settings,
            return_type: 'feature',
            devices: Math.max(Math.floor((settings.devices || 1) / length), 1),
            area,
            instance:
              area.properties?.name ||
              `${area.geometry.type}${area.id ? `-${area.id}` : ''}`,
            route_chunk_size: settings.route_chunk_size,
            last_seen: Math.floor(
              (settings.last_seen?.getTime?.() || 0) / 1000,
            ),
          }),
        })
          .then((res) => res.json())
          .then((r) => {
            const fetch_time = Date.now() - startTime
            setStatic('loading', (prev) => ({
              ...prev,
              [r.data?.properties?.name]: {
                ...r.stats,
                fetch_time,
              },
            }))
            console.log(area.properties?.name)
            Object.entries(r.stats).forEach(([k, v]) =>
              // eslint-disable-next-line no-console
              console.log(fromSnakeCase(k), v),
            )
            console.log(`Total Time: ${fetch_time / 1000}s\n`)
            console.log('-----------------')
            return r.data
          })
          .catch(() => {
            setStatic('loading', (prev) => ({
              ...prev,
              [area.properties?.name ||
              `${area.geometry.type}${area.id ? `-${area.id}` : ''}`]: false,
            }))
          })
      }),
  )
  setStatic('totalLoadingTime', Date.now() - totalStartTime)
  return {
    type: 'FeatureCollection',
    features: features.flatMap((r) =>
      r.status === 'fulfilled' && r.value ? r.value : [],
    ),
  }
}

export async function getMarkers(
  map: Map,
  data: UsePersist['data'],
  area: UseStatic['geojson'],
  enableStops: boolean,
  enableSpawnpoints: boolean,
  enableGyms: boolean,
  last_seen: UsePersist['last_seen'],
): Promise<Data> {
  const [pokestops, gyms, spawnpoints] = await Promise.all(
    [
      enableStops ? 'pokestop' : '',
      enableGyms ? 'gym' : '',
      enableSpawnpoints ? 'spawnpoint' : '',
    ].map(async (category) =>
      category &&
      (data === 'area'
        ? area.features.filter(
            (feature) =>
              feature.geometry.type === 'Polygon' ||
              feature.geometry.type === 'MultiPolygon',
          ).length
        : true)
        ? fetch(
            `/internal/data/${data === 'all' ? 'all' : 'area'}/${category}`,
            {
              method: 'POST',
              headers: {
                'Content-Type': 'application/json',
              },
              body: JSON.stringify({
                area:
                  data === 'area'
                    ? area.features.filter(
                        (feature) =>
                          feature.geometry.type === 'Polygon' ||
                          feature.geometry.type === 'MultiPolygon',
                      )
                    : data === 'bound'
                    ? getMapBounds(map)
                    : undefined,
                last_seen: Math.floor((last_seen?.getTime?.() || 0) / 1000),
              }),
            },
          ).then((res) => res.json())
        : [],
    ),
  )
  return {
    pokestops: Array.isArray(pokestops) ? pokestops : [],
    gyms: Array.isArray(gyms) ? gyms : [],
    spawnpoints: Array.isArray(spawnpoints) ? spawnpoints : [],
  }
}

export async function convert<T = Array<object> | object | string>(
  area: ToConvert,
  return_type: UsePersist['polygonExportMode'],
  simplify: UsePersist['simplifyPolygons'],
): Promise<T> {
  try {
    const data = await fetch('/api/v1/convert/data', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        area,
        return_type,
        simplify,
      }),
    })
    if (!data.ok) {
      throw new Error('Unable to convert')
    }
    return await data.json().then((r) => r.data)
  } catch (e) {
    console.error(e)
    return '' as unknown as T
  }
}

export async function save(url: string, code: string) {
  try {
    const res = await fetch(url, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({ area: JSON.parse(code) }),
    })
    if (!res.ok) {
      throw new Error('Unable to save')
    }
    return await res.json()
  } catch (e) {
    console.error(e)
  }
}
