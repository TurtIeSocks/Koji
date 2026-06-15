/* eslint-disable @typescript-eslint/no-explicit-any */
/* eslint-disable no-restricted-syntax */
/* eslint-disable import/no-extraneous-dependencies */
import simpleRestProvider from 'ra-data-simple-rest'
import { fetchUtils, type GetListResult, type GetListParams } from 'ra-core'
import { type RaRecord } from 'react-admin'
import { stringify } from 'querystring'

/**
 * react-admin dataProvider over the v2 API.
 *
 * Resource-name → v2-segment map. `geo: true` resources (geofences/routes) are
 * geometry-bearing: their v2 list/get-one return a GeoJSON FeatureCollection /
 * Feature, NOT row records — so we project each feature into a flat row record
 * (`{ id, name, mode, geometry, ... }`) for react-admin. The plain resources
 * (projects/properties/tile-servers) already return row records.
 *
 * TODO(v2-verify): the GeoJSON→row projection for geofence/route is LOSSY and
 * runtime-unverified:
 *   - v2 collapses the 12 legacy RDM modes to 4 (pokemon/fort/quest/unset), so a
 *     geofence/route `mode` shown/filtered in the admin is the collapsed value.
 *   - link fields (geofence_id on routes, projects/properties on geofences) ride
 *     the feature `properties` only if the v2 reader emits them there; otherwise
 *     they are absent in the list/edit forms.
 *   - the v2 geofence/route LIST handler ignores ?page/?sortBy/?q (returns the
 *     whole collection), so server-side pagination/sort/filter is a no-op there
 *     (total is computed client-side). The plain-resource list honors ?page/
 *     ?per_page only (sort/filter also ignored server-side by the macro).
 */
interface ResourceDef {
  seg: string
  geo: boolean
}

const RESOURCE_MAP: Record<string, ResourceDef> = {
  geofence: { seg: 'geofences', geo: true },
  route: { seg: 'routes', geo: true },
  project: { seg: 'projects', geo: false },
  property: { seg: 'properties', geo: false },
  tileserver: { seg: 'tile-servers', geo: false },
  plugins: { seg: 'plugins', geo: false },
}

const segFor = (resource: string): string =>
  RESOURCE_MAP[resource]?.seg ?? resource
const isGeo = (resource: string): boolean => !!RESOURCE_MAP[resource]?.geo

const httpClient = async (
  url: string,
  options?: fetchUtils.Options,
): Promise<{
  status: number
  headers: Headers
  body: string
  /* eslint-disable-next-line */
  json: any
}> => {
  const { status, json, headers, body } = await fetchUtils.fetchJson(
    url,
    options,
  )
  const newHeaders = new Headers()

  Object.entries(headers).forEach(([k, v]) => {
    newHeaders.set(k, v)
  })

  return { status, json, headers: newHeaders, body }
}

const defaultProvider = simpleRestProvider('/', httpClient)

/** Project a GeoJSON Feature into a flat react-admin row record. */
const featureToRecord = (feature: any): RaRecord => {
  const props = feature?.properties || {}
  const id = props.id ?? props.__id ?? feature?.id
  return {
    ...props,
    id,
    name: props.name ?? props.__name ?? `${id}`,
    mode: props.mode ?? props.__mode ?? 'unset',
    geometry: feature?.geometry,
    geo_type: feature?.geometry?.type,
  }
}

/** Unwrap a v2 ok-envelope `data` from an httpClient json body. */
const unwrap = (json: any): any =>
  json && json.status === 'ok' ? json.data : json

/** Records out of a v2 list response: GeoJSON features (geo) or `data` rows. */
const recordsFromList = (resource: string, json: any): RaRecord[] => {
  const data = unwrap(json)
  if (isGeo(resource)) {
    const features = (data?.features as any[]) || []
    return features.map(featureToRecord)
  }
  return (data as RaRecord[]) || []
}

const getList = async (
  resource: string,
  params: GetListParams,
): Promise<GetListResult> => {
  // v2 list query: 1-based ?page=, ?per_page=, plus ?sortBy=&order=&q= (honored
  // server-side only by some resources — see the module TODO).
  const { page, perPage } = params.pagination
  const queryParams: Record<string, any> = {
    ...params.filter,
    page,
    per_page: perPage,
    sortBy: params.sort.field,
    order: params.sort.order,
  }
  if (params.filter?.q) queryParams.q = params.filter.q
  const url = `/api/v2/${segFor(resource)}?${stringify(queryParams)}`

  const { json } = await httpClient(url, {
    headers: new Headers({
      'Content-Type': 'application/json',
      Accept: 'application/json',
    }),
  })

  const records = recordsFromList(resource, json)
  const meta = json?.meta
  return {
    data: records,
    // Geo resources return the whole collection (no meta) → count client-side.
    total: meta?.total ?? records.length,
    pageInfo: meta
      ? { hasNextPage: meta.has_next, hasPreviousPage: meta.has_prev }
      : undefined,
  }
}

/** Encode a react-admin id into a v2 item path segment.
 * Plugins use a composite `kind:name` id → `/{kind}/{name}`; everything else is
 * a single `/{id}` segment. */
const itemPath = (resource: string, id: RaRecord['id']): string => {
  const seg = segFor(resource)
  if (resource === 'plugins') {
    const [kind, ...rest] = `${id}`.split(':')
    return `/api/v2/${seg}/${kind}/${rest.join(':')}`
  }
  return `/api/v2/${seg}/${id}`
}

export const dataProvider: typeof defaultProvider = {
  ...defaultProvider,
  getList,
  getManyReference: getList,
  getMany: async (resource, params) => {
    // No v2 batch-by-ids endpoint: fetch each id in parallel, drop misses.
    const results = await Promise.allSettled(
      params.ids.map((id) =>
        httpClient(itemPath(resource, id)).then(({ json }) =>
          isGeo(resource)
            ? featureToRecord(
                (unwrap(json)?.features as any[])?.[0] ?? unwrap(json),
              )
            : (unwrap(json) as RaRecord),
        ),
      ),
    )
    return {
      data: results
        .filter(
          (r): r is PromiseFulfilledResult<RaRecord> =>
            r.status === 'fulfilled' && !!r.value,
        )
        .map((r) => r.value),
    } as any
  },
  getOne: (resource, params) =>
    httpClient(itemPath(resource, params.id)).then(({ json }) => {
      const data = unwrap(json)
      if (isGeo(resource)) {
        // get-one returns a single Feature (?format=feature default) — or a
        // FeatureCollection if the server defaulted to that. Normalize.
        const feature = data?.type === 'FeatureCollection' ? data.features?.[0] : data
        return { data: featureToRecord(feature) }
      }
      return { data }
    }),
  create: async (resource, params) => {
    const { json } = await httpClient(`/api/v2/${segFor(resource)}`, {
      method: 'POST',
      body: JSON.stringify(params.data),
    })
    const data = unwrap(json)
    return {
      data: { ...data, id: data && 'id' in data ? data.id : '0' },
    }
  },
  update: async (resource, params) => {
    return httpClient(itemPath(resource, params.id), {
      method: 'PATCH',
      body: JSON.stringify(params.data),
    }).then(({ json }) => {
      const data = unwrap(json)
      if (Array.isArray(data)) {
        return {
          data: data.find(
            (record: RaRecord) =>
              `${record.id || record.username}` === `${params.id}`,
          ),
        }
      }
      return { data: { ...data, id: data && 'id' in data ? data.id : params.id } }
    })
  },
  delete: (resource, params) =>
    // v2 DELETE → 204 No Content (no body). Synthesize `{ data: { id } }`.
    httpClient(itemPath(resource, params.id), {
      method: 'DELETE',
      headers: new Headers({
        'Content-Type': 'application/json',
        Accept: 'application/json',
      }),
    }).then(() => ({ data: { id: params.id } as RaRecord }) as any),
  deleteMany: async (resource, params) => {
    const results = await Promise.allSettled(
      params.ids.map((id) =>
        httpClient(itemPath(resource, id), {
          method: 'DELETE',
          headers: new Headers({
            'Content-Type': 'application/json',
            Accept: 'application/json',
          }),
        }).then(() => id),
      ),
    )
    return {
      data: results
        .filter((result) => result.status === 'fulfilled')
        .map((result) => (result as PromiseFulfilledResult<any>).value),
    }
  },
}
