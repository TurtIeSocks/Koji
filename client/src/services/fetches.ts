/* eslint-disable no-console */
/* eslint-disable no-nested-ternary */
import type {
  CombinedState,
  KojiResponse,
  PixiMarker,
  Conversions,
  Feature,
  FeatureCollection,
  DbOption,
} from '@assets/types'
import { UsePersist, usePersist } from '@hooks/usePersist'
import { UseStatic, useStatic } from '@hooks/useStatic'
import { useShapes } from '@hooks/useShapes'
import { UseDbCache, useDbCache } from '@hooks/useDbCache'

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
        networkStatus: {
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

export async function getKojiCache<T extends 'geofence' | 'project' | 'route'>(
  resource: T,
): Promise<UseDbCache[T] | null> {
  const res = await fetch(`/internal/admin/${resource}/all/`, {
    method: 'GET',
    headers: {
      'Content-Type': 'application/json',
    },
  })
  if (!res.ok) {
    useStatic.setState({
      networkStatus: {
        message: await res.text(),
        status: res.status,
        severity: 'error',
      },
    })
    return null
  }
  const { data }: KojiResponse<UseDbCache[T][string][]> = await res.json()
  const asObject = Object.fromEntries(data.map((d) => [d.id, d]))
  useDbCache.setState({ [resource]: asObject })
  return asObject
}

export async function getScannerCache() {
  return getData<KojiResponse<DbOption[]>>(
    '/internal/routes/from_scanner',
  ).then((res) => {
    if (res) {
      // eslint-disable-next-line no-console
      console.log('Cache set:', 'scanner', res.data.length)
      const asObject = Object.fromEntries(
        res.data.map((t) => [`${t.id}__${t.mode}__SCANNER`, t]),
      )
      useDbCache.setState({
        scanner: asObject,
      })
      return asObject
    }
  })
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
    save_to_scanner,
    skipRendering,
    last_seen,
    route_chunk_size,
  } = usePersist.getState()
  const { geojson, setStatic } = useStatic.getState()
  const { add, activeRoute } = useShapes.getState().setters
  const { getFromKojiKey, getRouteByCategory } = useDbCache.getState()

  activeRoute('layer_1')
  setStatic(
    'loading',
    Object.fromEntries(
      geojson.features
        .filter((feat) => feat.geometry.type.includes('Polygon'))
        .map((k) => [
          getFromKojiKey(k.id as string)?.name ||
            `${k.geometry.type}${k.id ? `-${k.id}` : ''}`,
          null,
        ]),
    ),
  )
  setStatic('totalLoadingTime', 0)

  const totalStartTime = Date.now()
  const features = await Promise.allSettled<Feature>(
    (geojson?.features || [])
      .filter((x) => x.geometry.type.includes('Polygon'))
      .map(async (area) => {
        const fenceRef = getFromKojiKey(area.id as string)
        const routeRef = getRouteByCategory(category, fenceRef?.name)
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
              area: {
                ...area,
                properties: {
                  __id: routeRef?.id,
                  __name: fenceRef?.name,
                  __geofence_id: fenceRef?.id,
                  __mode: fenceRef?.mode,
                },
              },
              instance:
                fenceRef?.name ||
                `${area.geometry.type}${area.id ? `-${area.id}` : ''}`,
              route_chunk_size,
              last_seen: Math.floor((last_seen?.getTime?.() || 0) / 1000),
              radius,
              min_points,
              fast,
              routing_time,
              only_unique,
              save_to_db,
              save_to_scanner,
            }),
          },
        )
        if (!res.ok) {
          if (fenceRef?.name) {
            setStatic('loading', (prev) => ({
              ...prev,
              [fenceRef?.name ||
              `${area.geometry.type}${area.id ? `-${area.id}` : ''}`]: false,
            }))
          }
          useStatic.setState({
            networkStatus: {
              message: await res.text(),
              status: res.status,
              severity: 'error',
            },
          })
          return null
        }
        const json = await res.json()
        const fetch_time = Date.now() - startTime
        if (fenceRef?.name) {
          setStatic('loading', (prev) => ({
            ...prev,
            [fenceRef?.name]: {
              ...json.stats,
              fetch_time,
            },
          }))
        }
        console.log(fenceRef?.name)
        Object.entries(json.stats).forEach(([k, v]) =>
          // eslint-disable-next-line no-console
          console.log(fromSnakeCase(k), v),
        )
        console.log(`Total Time: ${fetch_time / 1000}s\n`)
        console.log('-----------------')
        return json.data
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
  if (!skipRendering) add(features)
  if (save_to_db) await getKojiCache('route')
  if (save_to_scanner) await getScannerCache()
  return {
    type: 'FeatureCollection',
    features,
  }
}

export async function getMarkers(
  category: UsePersist['category'],
  bounds: ReturnType<typeof getMapBounds>,
  data: UsePersist['data'],
  area: UseStatic['geojson'],
  last_seen: UsePersist['last_seen'],
  signal: AbortSignal,
): Promise<PixiMarker[]> {
  try {
    const res = await fetch(`/internal/data/${data}/${category}`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      signal,
      body: JSON.stringify({
        area: data === 'area' ? area : undefined,
        ...(data === 'bound' && bounds),
        last_seen: Math.floor((last_seen?.getTime?.() || 0) / 1000),
      }),
    })
    if (!res.ok) {
      const message =
        (await res.text()) ||
        {
          400: 'Try refreshing the page or contacting the developer',
          401: 'Try refreshing the page and signing in again',
          404: 'Try refreshing the page or contacting the developer',
          408: 'Check CloudFlare or Nginx/Apache Settings',
          413: 'Check CloudFlare or Nginx/Apache Settings',
          500: 'Refresh the page, resetting the Kōji server, or contacting the developer',
          524: 'Check CloudFlare or Nginx/Apache Timeout Settings',
        }[res.status] ||
        ''
      useStatic.setState({
        networkStatus: {
          message,
          status: res.status,
          severity: 'error',
        },
      })
      throw new Error(message)
    }
    return await res.json()
  } catch (e) {
    if (e instanceof Error) {
      if (e.name !== 'AbortError' || process.env.NODE_ENV === 'development') {
        console.error(e)
      }
    }
    return []
  }
}

export async function convert<T = Conversions>(
  area: Conversions,
  return_type: UsePersist['polygonExportMode'],
  simplify: UsePersist['simplifyPolygons'],
  geometry_type?: UsePersist['geometryType'],
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
        geometry_type,
      }),
    })
    if (!res.ok) {
      useStatic.setState({
        networkStatus: {
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
        networkStatus: {
          message: await res.text(),
          status: res.status,
          severity: 'error',
        },
      })
      throw new Error('Unable to save')
    }
    useStatic.setState({
      networkStatus: {
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
