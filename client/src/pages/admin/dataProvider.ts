/* eslint-disable @typescript-eslint/no-explicit-any */
/* eslint-disable no-restricted-syntax */
/* eslint-disable import/no-extraneous-dependencies */
import simpleRestProvider from 'ra-data-simple-rest'
import { fetchUtils, type GetListResult, type GetListParams } from 'ra-core'
import { type RaRecord } from 'react-admin'
import { stringify } from 'querystring'

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

const getList = async (
  resource: string,
  params: GetListParams,
): Promise<GetListResult> => {
  const queryParams = {
    ...params.pagination,
    page: params.pagination.page - 1,
    sortBy: params.sort.field,
    order: params.sort.order,
  }
  const url = `/api/internal/admin/${resource}?${stringify(queryParams)}`

  const { json } = await httpClient(url, {
    headers: new Headers({
      'Content-Type': 'application/json',
      Accept: 'application/json',
    }),
  })
  return {
    data: json.data.results,
    total: json.data.total,
    pageInfo: {
      hasNextPage: json.data.has_next,
      hasPreviousPage: json.data.has_prev,
    },
  }
}

export const dataProvider: typeof defaultProvider = {
  ...defaultProvider,
  getMany: async (resource) => {
    const url = `/api/internal/admin/${resource}`
    const options = {}

    const { json } = await httpClient(url, options)
    return {
      data: json.data,
      total: json.total,
    }
  },
  getManyReference: getList,
  getList,
  getOne: (resource, params) =>
    httpClient(`/api/internal/admin/${resource}/${params.id}`).then(
      ({ json }) => {
        return resource === 'geofence'
          ? {
              data: {
                ...json.data,
                properties: Object.entries(
                  json.data?.area?.properties || {},
                ).map(([key, value]) => ({
                  key,
                  value,
                })),
              } as any,
            }
          : { data: json.data }
      },
    ),
  create: async (resource, params) => {
    const { json } = await httpClient(`/${resource}`, {
      method: 'POST',
      body: JSON.stringify({
        area: params.data.area,
        instance: params.data.name,
      }),
    })
    return {
      data: { ...json, id: 'id' in json ? json.id : '0' },
    }
  },
  update: (resource, params) =>
    httpClient(`/api/internal/admin/${resource}/${params.id}`, {
      method: 'PATCH',
      body: JSON.stringify(params.data),
    }).then(({ json }) => {
      if (Array.isArray(json)) {
        return {
          data: json.find(
            (record: RaRecord) =>
              `${record.id || record.username}` === `${params.id}`,
          ),
        }
      }

      return { data: { ...json, id: 'id' in json ? json.id : params.id } }
    }),
  delete: (resource, params) =>
    httpClient(`/api/internal/admin/${resource}/${params.id}`, {
      method: 'DELETE',
      headers: new Headers({
        'Content-Type': 'application/json',
        Accept: 'application/json',
      }),
    }).then(({ json }) => ({ data: json })),
  // deleteMany: async (resource, params) => {
  //   const results = await Promise.allSettled(
  //     params.ids.map((id) =>
  //       httpClient(`/api/internal/admin/${resource}/${id}`, {
  //         method: 'DELETE',
  //         headers: new Headers({
  //           'Content-Type': 'application/json',
  //           Accept: 'application/json',
  //         }),
  //       }).then(({ json }) => ({ data: json })),
  //     ),
  //   )
  //   return results
  //     .filter((result) => result.status === 'fulfilled')
  //     .map((result) => (result as PromiseFulfilledResult<{ data: any }>).value)
  // },
}
