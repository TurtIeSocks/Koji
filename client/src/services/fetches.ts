/* eslint-disable no-console */
/* eslint-disable no-nested-ternary */
import type {
  KojiResponse,
  PixiMarker,
  Conversions,
  Feature,
  FeatureCollection,
  DbOption,
  Category,
} from '@assets/types'
import { UsePersist, usePersist } from '@hooks/usePersist'
import { useStatic } from '@hooks/useStatic'
import { UseShapes, useShapes } from '@hooks/useShapes'
import { UseDbCache, useDbCache } from '@hooks/useDbCache'

import { fromSnakeCase, getRouteType } from './utils'

export async function fetchWrapper<T>(
  url: string,
  options: RequestInit = {},
): Promise<T | null> {
  try {
    const res = await fetch(url, options)
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
  console.log(
    'Cache set:',
    resource,
    process.env.NODE_ENV === 'development' ? data : data.length,
  )
  return asObject
}

export async function refreshKojiCache() {
  await Promise.allSettled([
    getKojiCache('geofence'),
    getKojiCache('project'),
    getKojiCache('route'),
  ])
}

export async function getScannerCache() {
  return fetchWrapper<KojiResponse<DbOption[]>>(
    '/internal/routes/from_scanner',
  ).then((res) => {
    if (res) {
      const asObject = Object.fromEntries(
        res.data.map((t) => [`${t.id}__${t.mode}__SCANNER`, t]),
      )
      useDbCache.setState({
        scanner: asObject,
      })
      console.log(
        'Cache set:',
        'scanner',
        process.env.NODE_ENV === 'development' ? res.data : res.data.length,
      )
      return asObject
    }
  })
}

export async function getFullCache() {
  Promise.all(
    (['geofence', 'route', 'project', 'scanner'] as const).map((resource) =>
      resource === 'scanner' ? getScannerCache() : getKojiCache(resource),
    ),
  )
}

export async function clusteringRouting(): Promise<FeatureCollection> {
  const {
    mode,
    radius,
    category,
    min_points,
    fast,
    route_split_level,
    only_unique,
    save_to_db,
    save_to_scanner,
    skipRendering,
    last_seen,
    sort_by,
    tth,
    bootstrap_mode,
  } = usePersist.getState()
  const { geojson, setStatic, bounds } = useStatic.getState()
  const { add, activeRoute } = useShapes.getState().setters
  const { getFromKojiKey, getRouteByCategory } = useDbCache.getState()

  const areas = (geojson?.features || []).filter((x) =>
    x.geometry.type.includes('Polygon'),
  )

  if (!areas.length) {
    areas.push({
      id: 'bounds',
      type: 'Feature',
      geometry: {
        type: 'Polygon',
        coordinates: [
          [
            [bounds.min_lon, bounds.min_lat],
            [bounds.max_lon, bounds.min_lat],
            [bounds.max_lon, bounds.max_lat],
            [bounds.min_lon, bounds.max_lat],
            [bounds.min_lon, bounds.min_lat],
          ],
        ],
      },
      properties: {
        __name: 'bounds',
        __mode: getRouteType(category),
      },
    })
  }

  activeRoute('layer_1')
  useStatic.setState({
    loading: Object.fromEntries(
      areas.map((k) => [
        getFromKojiKey(k.id as string)?.name ||
          `${k.geometry.type}${k.id ? `-${k.id}` : ''}`,
        null,
      ]),
    ),
    loadingAbort: Object.fromEntries(
      areas.map((k) => [
        getFromKojiKey(k.id as string)?.name ||
          `${k.geometry.type}${k.id ? `-${k.id}` : ''}`,
        new AbortController(),
      ]),
    ),
    totalLoadingTime: 0,
  })

  const totalStartTime = Date.now()
  const features = await Promise.allSettled<Feature>(
    areas.map(async (area) => {
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
          signal:
            useStatic.getState().loadingAbort[
              getFromKojiKey(area.id as string)?.name ||
                `${area.geometry.type}${area.id ? `-${area.id}` : ''}`
            ]?.signal,
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
            last_seen: Math.floor((last_seen?.getTime?.() || 0) / 1000),
            radius,
            min_points,
            fast,
            only_unique,
            save_to_db,
            save_to_scanner,
            route_split_level,
            sort_by,
            tth,
            bootstrap_mode:
              typeof bootstrap_mode === 'string' ? 0 : bootstrap_mode,
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
      setStatic('loading', (prev) => ({
        ...prev,
        [fenceRef
          ? fenceRef?.name
          : `${area.geometry.type}${area.id ? `-${area.id}` : ''}`]: {
          ...json.stats,
          fetch_time,
        },
      }))
      console.log(fenceRef?.name)
      Object.entries(json.stats).forEach(([k, v]) =>
        // eslint-disable-next-line no-console
        console.log(fromSnakeCase(k), v),
      )
      console.log(`Total Time: ${fetch_time / 1000}s\n`)
      console.log('-----------------')
      return {
        id: `${area.id.toString().split('__')[0]}__${getRouteType(category)}__${
          fenceRef || routeRef ? 'KOJI' : 'CLIENT'
        }}`,
        ...json.data,
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
  if (!skipRendering) add(features)
  if (save_to_db) await getKojiCache('route')
  if (save_to_scanner) await getScannerCache()
  return {
    type: 'FeatureCollection',
    features,
  }
}

export async function getMarkers(
  signal: AbortSignal,
  category: Category,
): Promise<PixiMarker[]> {
  const { data, last_seen } = usePersist.getState()
  const { geojson, bounds } = useStatic.getState()
  if (data === 'area' && !geojson.features.length) return []
  try {
    const res = await fetch(`/internal/data/${data}/${category}`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      signal,
      body: JSON.stringify({
        area:
          data === 'area'
            ? {
                ...geojson,
                features: geojson.features.filter((feature) =>
                  feature.geometry.type.includes('Polygon'),
                ),
              }
            : undefined,
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

export async function save(
  url: string,
  code: string,
): Promise<{ updates: number; inserts: number } | null> {
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
    const json: KojiResponse<{ updates: number; inserts: number }> =
      await res.json()
    useStatic.setState({
      networkStatus: {
        message: `Saved successfully`,
        status: res.status,
        severity: 'success',
      },
    })
    return json.data
  } catch (e) {
    console.error(e)
    return null
  }
}

export async function s2Coverage(id: Feature['id'], lat: number, lon: number) {
  const { s2cells, radius, fillCoveredCells } = usePersist.getState()
  if (fillCoveredCells) {
    const s2cellCoverage: UseShapes['s2cellCoverage'] = Object.fromEntries(
      Object.entries(useShapes.getState().s2cellCoverage).filter(
        ([, v]) => v !== id.toString(),
      ),
    )
    await Promise.allSettled(
      s2cells.map(async (level) =>
        fetchWrapper<KojiResponse<string[]>>('/api/v1/s2/circle-coverage', {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
          },
          body: JSON.stringify({ lat, lon, radius, level }),
        }).then((res) => {
          if (res) {
            res.data.forEach((cell) => {
              if (!s2cellCoverage[cell]) s2cellCoverage[cell] = id.toString()
            })
          }
        }),
      ),
    )
    return s2cellCoverage
  }
  return {}
}
