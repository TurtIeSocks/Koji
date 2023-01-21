/* eslint-disable no-console */
/* eslint-disable no-nested-ternary */
import type { Map } from 'leaflet'
import type { Feature, FeatureCollection } from 'geojson'

import type { CombinedState, Data, ToConvert } from '@assets/types'
import { UsePersist, usePersist } from '@hooks/usePersist'
import { UseStatic, useStatic } from '@hooks/useStatic'
import { useShapes } from '@hooks/useShapes'

import { fromSnakeCase, getMapBounds } from './utils'

export async function getData<T>(
  url: string,
  options: RequestInit = {},
  settings: CombinedState & { area?: Feature } = {},
): Promise<T | null> {
  try {
    const res = Object.keys(settings).length
      ? await fetch(url, {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
          },
          body: JSON.stringify(settings),
        })
      : await fetch(url, options)
    if (!res.ok) {
      useStatic.setState({
        networkError: {
          message: await res.text(),
          status: res.status,
          severity: 'error',
        },
      })
    }
    return await res.json()
  } catch (e) {
    console.error(e)
    return null
  }
}

export async function clusteringRouting(): Promise<FeatureCollection> {
  const {
    mode,
    radius,
    category,
    min_points,
    fast,
    routing_time,
    only_unique,
    save_to_db,
    last_seen,
    route_chunk_size,
  } = usePersist.getState()
  const { geojson, setStatic } = useStatic.getState()
  const { add, activeRoute } = useShapes.getState().setters
  activeRoute('layer_1')
  setStatic(
    'loading',
    Object.fromEntries(
      geojson.features
        .filter(
          (feat) =>
            feat.geometry.type === 'Polygon' ||
            feat.geometry.type === 'MultiPolygon',
        )
        .map((k) => [
          k.properties?.__name || `${k.geometry.type}${k.id ? `-${k.id}` : ''}`,
          null,
        ]),
    ),
  )
  setStatic('totalLoadingTime', 0)

  const totalStartTime = Date.now()
  const features = await Promise.allSettled<Feature>(
    (geojson?.features || [])
      .filter(
        (x) =>
          x.geometry.type === 'Polygon' || x.geometry.type === 'MultiPolygon',
      )
      .map(async (area) => {
        const startTime = Date.now()
        const res = await fetch(
          mode === 'bootstrap'
            ? '/api/v1/calc/bootstrap'
            : `/api/v1/calc/${mode}/${category}`,
          {
            method: 'POST',
            headers: {
              'Content-Type': 'application/json',
            },
            body: JSON.stringify({
              return_type: 'feature',
              area,
              instance:
                area.properties?.__name ||
                `${area.geometry.type}${area.id ? `-${area.id}` : ''}`,
              route_chunk_size,
              last_seen: Math.floor((last_seen?.getTime?.() || 0) / 1000),
              radius,
              min_points,
              fast,
              routing_time,
              only_unique,
              save_to_db,
            }),
          },
        )
        if (!res.ok) {
          setStatic('loading', (prev) => ({
            ...prev,
            [area.properties?.__name ||
            `${area.geometry.type}${area.id ? `-${area.id}` : ''}`]: false,
          }))
          console.log(res)
          useStatic.setState({
            networkError: {
              message: await res.text(),
              status: res.status,
              severity: 'error',
            },
          })
          return null
        }
        const json = await res.json()
        const fetch_time = Date.now() - startTime
        setStatic('loading', (prev) => ({
          ...prev,
          [json.data?.properties?.__name]: {
            ...json.stats,
            fetch_time,
          },
        }))
        console.log(area.properties?.__name)
        Object.entries(json.stats).forEach(([k, v]) =>
          // eslint-disable-next-line no-console
          console.log(fromSnakeCase(k), v),
        )
        console.log(`Total Time: ${fetch_time / 1000}s\n`)
        console.log('-----------------')
        return {
          ...json.data,
          properties: {
            ...json.data.properties,
            __geofence_id: area.properties?.__koji_id,
          },
        }
      }),
  ).then((feats) =>
    feats
      .filter(
        (f): f is PromiseFulfilledResult<Feature> =>
          f.status === 'fulfilled' && !!f.value,
      )
      .map((f) => f.value),
  )

  setStatic('totalLoadingTime', Date.now() - totalStartTime)
  add(features)

  return {
    type: 'FeatureCollection',
    features,
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
  const filtered = area.features.filter(
    (feature) =>
      feature.geometry.type === 'Polygon' ||
      feature.geometry.type === 'MultiPolygon',
  )
  const [pokestops, gyms, spawnpoints] = await Promise.all(
    [
      enableStops ? 'pokestop' : '',
      enableGyms ? 'gym' : '',
      enableSpawnpoints ? 'spawnpoint' : '',
    ].map(async (category) => {
      if (category && (data === 'area' ? filtered.length : true)) {
        const res = await fetch(`/internal/data/${data}/${category}`, {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
          },
          body: JSON.stringify({
            area: data === 'area' ? filtered : undefined,
            ...(data === 'bound' && getMapBounds(map)),
            last_seen: Math.floor((last_seen?.getTime?.() || 0) / 1000),
          }),
        })
        if (!res.ok) {
          useStatic.setState({
            networkError: {
              message: await res.text(),
              status: res.status,
              severity: 'error',
            },
          })
          return null
        }
        return res.json()
      }
      return []
    }),
  )
  return {
    pokestops: Array.isArray(pokestops) ? pokestops : [],
    gyms: Array.isArray(gyms) ? gyms : [],
    spawnpoints: Array.isArray(spawnpoints) ? spawnpoints : [],
  }
}

export async function convert<T = ToConvert>(
  area: ToConvert,
  return_type: UsePersist['polygonExportMode'],
  simplify: UsePersist['simplifyPolygons'],
  url = '/api/v1/convert/data',
): Promise<T> {
  try {
    const res = await fetch(url, {
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
    if (!res.ok) {
      useStatic.setState({
        networkError: {
          message: await res.text(),
          status: res.status,
          severity: 'error',
        },
      })
      throw new Error('Unable to convert')
    }
    return await res.json().then((r) => r.data)
  } catch (e) {
    console.error(e)
    return '' as unknown as T
  }
}

export async function save<T>(url: string, code: string): Promise<T | null> {
  try {
    const res = await fetch(url, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({ area: JSON.parse(code) }),
    })
    if (!res.ok) {
      useStatic.setState({
        networkError: {
          message: await res.text(),
          status: res.status,
          severity: 'error',
        },
      })
      throw new Error('Unable to save')
    }
    useStatic.setState({
      networkError: {
        message: 'Saved successfully!',
        status: res.status,
        severity: 'success',
      },
    })
    return await res.json()
  } catch (e) {
    console.error(e)
    return null
  }
}
