/* eslint-disable no-nested-ternary */
import type { Map } from 'leaflet'
import type { FeatureCollection } from 'geojson'

import type { CombinedState, Data, ToConvert } from '@assets/types'
import type { UseStore } from '@hooks/useStore'
import type { UseStatic } from '@hooks/useStatic'

import { fromSnakeCase, getMapBounds } from './utils'

export async function getData<T>(
  url: string,
  settings: CombinedState & { area?: [number, number][] } = {},
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
    // eslint-disable-next-line no-console
    console.error(e)
    return null
    // return { error: e instanceof Error ? e.message : 'Unknown Error' }
  }
}

export async function getLotsOfData(
  url: string,
  settings: CombinedState = {},
): Promise<FeatureCollection> {
  const { length = 0 } = settings.geojson?.features || {}
  const features = await Promise.allSettled(
    (settings.geojson?.features || [])
      .filter(
        (x) =>
          x.geometry.type === 'Polygon' || x.geometry.type === 'MultiPolygon',
      )
      .map((area) =>
        fetch(url, {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
          },
          body: JSON.stringify({
            ...settings,
            return_type: 'feature',
            devices: Math.max(Math.floor((settings.devices || 1) / length), 1),
            area,
            instance: area.properties?.name,
            route_chunk_size: settings.route_chunk_size,
            last_seen: Math.floor(
              (settings.last_seen?.getTime?.() || 0) / 1000,
            ),
          }),
        })
          .then((res) => res.json())
          .then((r) => {
            Object.entries(r.stats).forEach(([k, v]) =>
              // eslint-disable-next-line no-console
              console.log(fromSnakeCase(k), v),
            )
            return r.data
          }),
      ),
  )
  return {
    type: 'FeatureCollection',
    features: features.flatMap((r) =>
      r.status === 'fulfilled' ? r.value : [],
    ),
  }
}

export async function getMarkers(
  map: Map,
  data: UseStore['data'],
  area: UseStatic['geojson'],
  enableStops: boolean,
  enableSpawnpoints: boolean,
  enableGyms: boolean,
  last_seen: UseStore['last_seen'],
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
        ? fetch(`/api/data/${data === 'all' ? 'all' : 'area'}/${category}`, {
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
          }).then((res) => res.json())
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
  return_type: UseStore['polygonExportMode'],
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
      }),
    })
    return await data.json().then((r) => r.data)
  } catch (e) {
    // eslint-disable-next-line no-console
    console.error(e)
    return '' as unknown as T
  }
}
