/* eslint-disable no-console */
/* eslint-disable no-nested-ternary */
import type {
  ApiEnvelope,
  JobRecord,
  CalcJobResult,
  PixiMarker,
  Conversions,
  Feature,
  FeatureCollection,
  DbOption,
  Category,
  S2Response,
} from '@assets/types'
import { UsePersist, usePersist } from '@hooks/usePersist'
import { useStatic } from '@hooks/useStatic'
import { UseShapes, useShapes } from '@hooks/useShapes'
import { UseDbCache, useDbCache } from '@hooks/useDbCache'

import { fromSnakeCase, getMapBounds, getRouteType } from './utils'

/**
 * v2 envelope adapter. Every `/api/v2/*` JSON response is the discriminated
 * `{ status: 'ok', data, meta? } | { status: 'error', error }` envelope, so
 * `fetchWrapper<T>` returns the UNWRAPPED `data` (`T`) on success, or `null` on
 * error (pushing `error.message` as a notification). Callers no longer read
 * `res.data` — `T` is the inner payload type directly.
 *
 * Note: raw `?format=` exports (e.g. `sql`/`text`/`poracle`) come back NOT
 * enveloped — JSON.parse would throw on raw text. Those specific calls (see
 * `convert`) read the body themselves rather than going through `fetchWrapper`.
 */
export async function fetchWrapper<T>(
  url: string,
  options: RequestInit = {},
): Promise<T | null> {
  try {
    const res = await fetch(url, options)
    // Parse the body once; the v2 envelope discriminates on `status`.
    const json = (await res.json().catch(() => null)) as ApiEnvelope<T> | null
    if (!res.ok || (json && json.status === 'error')) {
      const message =
        json && json.status === 'error'
          ? json.error?.message || 'Request failed'
          : `Request failed (${res.status})`
      useStatic.setState({
        notification: {
          message,
          status: res.status,
          severity: 'error',
        },
      })
      return null
    }
    if (json && json.status === 'ok') return json.data
    // 204 No Content / empty body → no data to unwrap.
    return null
  } catch (e) {
    console.error(e)
    return null
  }
}

/**
 * Poll a v2 job to a terminal state. Loops `GET /api/v2/jobs/{id}?wait=10`
 * (the backend long-polls up to 10s per call, then returns the record regardless
 * — never a 504), re-issuing until `status` is terminal (`succeeded`/`failed`/
 * `canceled`) or the `signal` aborts. On abort it fires `DELETE /api/v2/jobs/{id}`
 * (best-effort cancellation) and rethrows the AbortError.
 *
 * TODO(v2-verify): the poll loop + `?wait=` long-poll are RUNTIME-UNVERIFIED here
 * (no backend/DB in this env). Confirm against a live deploy: terminal detection,
 * abort→DELETE, and that a `failed` job surfaces its `error` to the user.
 */
export async function pollJob(
  jobId: string,
  signal?: AbortSignal,
): Promise<JobRecord> {
  // eslint-disable-next-line no-constant-condition
  while (true) {
    if (signal?.aborted) {
      // Best-effort server-side cancel, then propagate the abort.
      fetchWrapper(`/api/v2/jobs/${jobId}`, { method: 'DELETE' }).catch(() => {})
      throw new DOMException('Aborted', 'AbortError')
    }
    const record = await fetchWrapper<JobRecord>(
      `/api/v2/jobs/${jobId}?wait=10`,
      { method: 'GET', signal },
    )
    if (!record) {
      // A null here is an envelope error / network failure (already notified).
      throw new Error(`Job ${jobId} could not be read`)
    }
    if (
      record.status === 'succeeded' ||
      record.status === 'failed' ||
      record.status === 'canceled'
    ) {
      return record
    }
    // Non-terminal: loop again immediately (the server already waited ~10s).
  }
}

/** v2 resource path segment for a cache key (`geofence`→`geofences`, …). */
const V2_RESOURCE_SEG: Record<'geofence' | 'project' | 'route', string> = {
  geofence: 'geofences',
  project: 'projects',
  route: 'routes',
}

/**
 * Refresh one Kōji metadata cache from v2.
 *
 * - `project` → `GET /api/v2/projects?per_page=9999` returns ROW records
 *   (`{ id, name, scanner, ... }`) — used as-is.
 * - `geofence`/`route` → `GET /api/v2/{seg}?per_page=9999` returns a GeoJSON
 *   `FeatureCollection` (these resources are geometry-bearing in v2). We derive a
 *   best-effort `DbOption` from each feature's `properties` (KojiMeta `id`/`name`/
 *   `mode`) + geometry type.
 *
 * TODO(v2-verify): the geofence/route cache is a LOSSY GeoJSON→DbOption derivation:
 *   (1) v2 collapses the 12 legacy RDM modes to 4 (`pokemon`/`fort`/`quest`/`unset`),
 *       so `DbOption.mode` is no longer the granular value `getRouteByCategory`
 *       (which matches `mode.includes('raid'|'quest'|'station')`) expects —
 *       route↔category matching will mis-resolve.
 *   (2) the row-link fields (`geofences`/`projects`/`geofence_id`) are NOT in the
 *       GeoJSON, so they default empty/undefined — `SelectProject`/`Instance`
 *       project→fence expansion loses those links.
 * Needs a dedicated v2 row/metadata endpoint (or `properties` carrying the links)
 * to be fully correct. Flagged for the user's live smoke.
 */
export async function getKojiCache<T extends 'geofence' | 'project' | 'route'>(
  resource: T,
): Promise<UseDbCache[T] | null> {
  const seg = V2_RESOURCE_SEG[resource]

  if (resource === 'project') {
    const data = await fetchWrapper<UseDbCache[T][string][]>(
      `/api/v2/${seg}?per_page=9999`,
      { method: 'GET', headers: { 'Content-Type': 'application/json' } },
    )
    if (!data) return null
    const asObject = Object.fromEntries(
      data.map((d) => [d.id, d]),
    ) as UseDbCache[T]
    useDbCache.setState({ [resource]: asObject })
    console.log(
      'Cache set:',
      resource,
      process.env.NODE_ENV === 'development' ? data : data.length,
    )
    return asObject
  }

  // geofence / route: GeoJSON FeatureCollection → best-effort DbOption[].
  const fc = await fetchWrapper<FeatureCollection>(
    `/api/v2/${seg}?per_page=9999`,
    { method: 'GET', headers: { 'Content-Type': 'application/json' } },
  )
  if (!fc) return null
  const records = (fc.features || []).map((feature) => {
    const props = feature.properties || {}
    const id = (props.id as number) ?? (props.__id as number) ?? feature.id
    return {
      id,
      name: (props.name as string) ?? (props.__name as string) ?? `${id}`,
      mode: (props.mode as DbOption['mode']) ?? props.__mode ?? 'unset',
      geo_type: feature.geometry?.type,
      geofence_id:
        (props.geofence_id as number) ?? (props.__geofence_id as number),
    } as DbOption
  })
  const asObject = Object.fromEntries(
    records.map((d) => [d.id, d]),
  ) as UseDbCache[T]
  useDbCache.setState({ [resource]: asObject })
  console.log(
    'Cache set:',
    resource,
    process.env.NODE_ENV === 'development' ? records : records.length,
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

/**
 * Scanner-sourced routes cache.
 *
 * TODO(v2-gap): the v1 `/internal/routes/from_scanner` endpoint (routes pulled
 * from the scanner DB's `instance`/`area` tables) has NO v2 equivalent — the v2
 * surface does not expose scanner-sourced routes. Stubbed to clear the `scanner`
 * cache so callers (`SaveToScanner`, the calc `save_to_scanner` refresh, the
 * `Instance` selector's scanner mode) build + run without crashing; they simply
 * see an empty scanner list. Flagged for the user — if scanner-route browsing is
 * still needed, a v2 endpoint must be added (P7+).
 */
export async function getScannerCache(): Promise<
  Record<string, DbOption> | undefined
> {
  const asObject: Record<string, DbOption> = {}
  useDbCache.setState({ scanner: asObject })
  console.log('Cache set:', 'scanner', '(v2-gap: stubbed empty)')
  return asObject
}

export async function getFullCache() {
  Promise.all(
    (['geofence', 'route', 'project', 'scanner'] as const).map((resource) =>
      resource === 'scanner' ? getScannerCache() : getKojiCache(resource),
    ),
  )
}

export async function clusteringRouting({
  feature,
  parent,
}: {
  feature?: Feature
  parent?: string | number
} = {}): Promise<FeatureCollection> {
  const {
    mode,
    radius,
    center_clusters,
    cluster_mode,
    category: rawCategory,
    min_points,
    fast,
    route_split_level,
    cluster_split_level,
    save_to_db,
    save_to_scanner,
    skipRendering,
    last_seen: raw,
    sort_by,
    tth,
    calculation_mode,
    s2_level,
    s2_size,
    max_clusters,
    routing_args,
    clustering_args,
    bootstrapping_args,
    genetic_post_processing,
    dev,
  } = usePersist.getState()
  const { geojson, setStatic, bounds } = useStatic.getState()
  const { add, activeRoute } = useShapes.getState().setters
  const { getFromKojiKey, getRouteByCategory } = useDbCache.getState()

  const areas = ((feature ? [feature] : geojson?.features) || []).filter(
    (x) =>
      x.geometry.type.includes('Polygon') &&
      x.geometry.type !== 'GeometryCollection' &&
      x.geometry.coordinates.length,
  )
  const last_seen = typeof raw === 'string' ? new Date(raw) : raw
  const category = rawCategory === 'fort' ? 'gym' : rawCategory

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

  activeRoute('0__unset__CLIENT')
  const totalStartTime = Date.now()

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
    totalStartTime,
    totalLoadingTime: 0,
  })

  const features = await Promise.allSettled<Feature | null>(
    areas.map(async (area) => {
      const fenceRef = getFromKojiKey(area.id as string)
      const routeRef = getRouteByCategory(category, fenceRef?.name)
      const startTime = Date.now()

      const instance =
        fenceRef?.name ||
        `${area.geometry.type}${area.id ? `-${area.id}` : ''}`
      const signal = useStatic.getState().loadingAbort[instance]?.signal

      // v2 calc body: a tagged CalcJobRequest. `mode` selects the op
      // (cluster | bootstrap — clusteringRouting only drives these two; reroute /
      // routeStats are separate call sites), `category` + the nested camelCase
      // arg-groups carry the former flat v1 fields.
      // TODO(v2-verify): the whole async job path here is RUNTIME-UNVERIFIED (no
      // backend/DB). Confirm against a live deploy: POST /jobs → 202 {job_id},
      // the nested arg-group field mapping below, and the result/stats shape read
      // out of the terminal JobRecord.
      const body =
        mode === 'bootstrap'
          ? {
              mode: 'bootstrap',
              category,
              instance,
              parent,
              area: parent ? undefined : area,
              bootstrap: {
                calculationMode: calculation_mode,
                radius,
                s2Level: s2_level,
                s2Size: s2_size,
                pluginArgs: bootstrapping_args || undefined,
              },
              routing: {
                sortBy: sort_by,
                routeSplitLevel: route_split_level,
                pluginArgs: routing_args || undefined,
              },
              output: { returnType: 'feature', saveToDb: save_to_db, saveToScanner: save_to_scanner },
              dev,
            }
          : {
              mode: 'cluster',
              category,
              instance,
              parent,
              area: parent ? undefined : area,
              clustering: {
                radius,
                minPoints: min_points,
                maxClusters: max_clusters,
                mode: cluster_mode,
                calculationMode: calculation_mode,
                s2Level: s2_level,
                s2Size: s2_size,
                clusterSplitLevel: cluster_split_level,
                centerClusters: center_clusters,
                geneticPostProcessing: genetic_post_processing,
                pluginArgs: clustering_args || undefined,
              },
              routing: {
                sortBy: sort_by,
                routeSplitLevel: route_split_level,
                pluginArgs: routing_args || undefined,
              },
              output: { returnType: 'feature', saveToDb: save_to_db, saveToScanner: save_to_scanner },
              dataFilter: {
                lastSeen: Math.floor((last_seen?.getTime?.() || 0) / 1000),
                tth,
              },
              dev,
            }

      // `fast` (v1 flag) has no v2 arg-group field — the adaptive-partition path
      // is now controlled by `dev.bypassAdaptivePartition`. Dropped here.
      // TODO(v2-verify): confirm dropping the v1 `fast` flag is intended.

      // Enqueue the job, then long-poll it to a terminal state.
      const enqueue = await fetchWrapper<{ job_id: string }>('/api/v2/jobs', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        signal,
        body: JSON.stringify(body),
      })
      let record: JobRecord | null = null
      if (enqueue?.job_id) {
        try {
          record = await pollJob(enqueue.job_id, signal)
        } catch (e) {
          if (!(e instanceof DOMException && e.name === 'AbortError')) {
            console.error(e)
          }
        }
      }

      if (!record || record.status !== 'succeeded' || !record.result) {
        if (fenceRef?.name) {
          setStatic('loading', (prev) => ({
            ...prev,
            [instance]: false,
          }))
        }
        if (record?.status === 'failed' && record.error) {
          useStatic.setState({
            notification: {
              message: record.error,
              status: 500,
              severity: 'error',
            },
          })
        }
        return null
      }

      const result: CalcJobResult = record.result
      const stats = result.stats
      const fetch_time = Date.now() - startTime
      setStatic('loading', (prev) => ({
        ...prev,
        [fenceRef
          ? fenceRef?.name
          : `${area.geometry.type}${area.id ? `-${area.id}` : ''}`]: {
          ...stats,
          fetch_time,
        },
      }))
      console.log(fenceRef?.name)
      Object.entries(stats || {}).forEach(([k, v]) =>
        // eslint-disable-next-line no-console
        console.log(fromSnakeCase(k), v),
      )
      const newId = `${
        routeRef ? routeRef.id : area.id.toString().split('__')[0]
      }__${getRouteType(category)}__${fenceRef || routeRef ? 'KOJI' : 'CLIENT'}`
      console.log(`Total Time: ${fetch_time / 1000}s\n`)
      console.log('-----------------')
      // v2 calc returns a FeatureCollection (geojson FC of the cluster centers);
      // v1 with return_type:'feature' returned a single Feature. Extract the lone
      // feature so the downstream `add()`/shape store still gets a Feature.
      // TODO(v2-verify): confirm the result FC carries exactly one MultiPoint
      // feature for the cluster/bootstrap centers (so [0] is correct).
      const resultFeature = result.data?.features?.[0]
      if (!resultFeature) return null
      return {
        ...resultFeature,
        id: newId,
        properties: {
          ...resultFeature.properties,
          __geofence_id: fenceRef?.id || undefined,
        },
      } as Feature
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
  setStatic('totalStartTime', 0)
  if (!skipRendering) add(features.filter((f) => !!f.geometry))
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
  tth: UsePersist['tth'],
): Promise<PixiMarker[]> {
  const { data, last_seen: raw } = usePersist.getState()
  const { geojson, bounds } = useStatic.getState()
  if (data === 'area' && !geojson.features.length) return []
  const last_seen = typeof raw === 'string' ? new Date(raw) : raw
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
        tth,
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
        notification: {
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
        notification: {
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
        notification: {
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
      notification: {
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

export async function getS2Cells(
  map: L.Map,
  level: number,
  signal: AbortSignal,
) {
  const { s2cellCoverage } = useShapes.getState()
  const { s2DisplayMode } = usePersist.getState()
  if (s2DisplayMode === 'none') return []

  return fetchWrapper<KojiResponse<S2Response[]>>(`/api/v1/s2/${level}`, {
    method: 'POST',
    body: JSON.stringify({
      ...getMapBounds(map),
      // ids: s2DisplayMode === 'all' ? undefined : Object.keys(s2cellCoverage),
    }),
    headers: {
      'Content-Type': 'application/json',
    },
    signal,
  }).then((res) => {
    if (res) {
      if (res.data.length >= 20_000) {
        useStatic.setState({
          notification: {
            message: `Loaded the maximum of ${Number(
              20_000,
            ).toLocaleString()} Level ${level} S2 cells`,
            severity: 'warning',
            status: 200,
          },
        })
        return res.data.filter(
          (c, i) => s2cellCoverage[c.id]?.length || i <= 20_000,
        )
      }
      return res.data
    }
  })
}

export async function s2Coverage(id: string, lat: number, lon: number) {
  const {
    s2cells,
    radius,
    s2_level: bootstrap_level,
    calculation_mode,
    s2_size: bootstrap_size,
    s2DisplayMode,
  } = usePersist.getState()
  if (s2DisplayMode !== 'none') {
    const s2cellCoverage: UseShapes['s2cellCoverage'] = Object.fromEntries(
      Object.entries(useShapes.getState().s2cellCoverage).map(([k, v]) => [
        k,
        v.filter((i) => i !== id),
      ]),
    )

    await Promise.allSettled(
      (calculation_mode === 'Radius' ? s2cells : [bootstrap_level]).map(
        async (level) =>
          fetchWrapper<KojiResponse<string[]>>(
            `/api/v1/s2/${
              calculation_mode === 'Radius' ? 'circle' : 'cell'
            }-coverage`,
            {
              method: 'POST',
              headers: {
                'Content-Type': 'application/json',
              },
              body: JSON.stringify({
                lat,
                lon,
                radius: calculation_mode === 'Radius' ? radius : undefined,
                size: calculation_mode === 'S2' ? bootstrap_size : undefined,
                level,
              }),
            },
          ).then((res) => {
            if (res) {
              res.data.forEach((c) => {
                if (s2cellCoverage[c]) {
                  s2cellCoverage[c] = [...s2cellCoverage[c], id.toString()]
                } else {
                  s2cellCoverage[c] = [id.toString()]
                }
              })
            }
          }),
      ),
    )
    return s2cellCoverage
  }
  return {}
}
