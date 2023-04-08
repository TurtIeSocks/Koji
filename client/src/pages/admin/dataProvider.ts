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
    ...params.filter,
    ...params.pagination,
    page: params.pagination.page - 1,
    sortBy: params.sort.field,
    order: params.sort.order,
  }
  const url = `/internal/admin/${resource}/?${stringify(queryParams)}`

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
    const url = `/internal/admin/${resource}/all/`
    const options = {}
    const { json } = await httpClient(url, options)
    return json
  },
  getManyReference: getList,
  getList,
  getOne: (resource, params) =>
    httpClient(`/internal/admin/${resource}/${params.id}/`).then(({ json }) => {
      // console.log(json)
      return json
    }),
  create: async (resource, params) => {
    const { json } = await httpClient(`/internal/admin/${resource}/`, {
      method: 'POST',
      body: JSON.stringify({
        ...params.data,
        id: 0,
        created_at: params.data.created_at || new Date(),
        updated_at: params.data.updated_at || new Date(),
      }),
    })
    return {
      data: { ...json, id: 'id' in json ? json.id : '0' },
    }
  },
  update: async (resource, params) => {
    return httpClient(`/internal/admin/${resource}/${params.id}/`, {
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
    })
  },
  delete: (resource, params) =>
    httpClient(`/internal/admin/${resource}/${params.id}/`, {
      method: 'DELETE',
      headers: new Headers({
        'Content-Type': 'application/json',
        Accept: 'application/json',
      }),
    }).then(({ json }) => ({ data: json })),
  deleteMany: async (resource, params) => {
    const results = await Promise.allSettled(
      params.ids.map((id) =>
        httpClient(`/internal/admin/${resource}/${id}/`, {
          method: 'DELETE',
          headers: new Headers({
            'Content-Type': 'application/json',
            Accept: 'application/json',
          }),
        }).then(({ json }) => ({ data: json })),
      ),
    )
    return {
      data: results
        .filter((result) => result.status === 'fulfilled')
        .map(
          (result) =>
            (result as PromiseFulfilledResult<{ data: any }>).value.data,
        ),
    }
  },
}
