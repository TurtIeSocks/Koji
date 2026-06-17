import * as React from 'react'
import ExportIcon from '@mui/icons-material/ImportExport'
import {
  Button,
  useListContext,
  useRecordContext,
  useUnselectAll,
} from 'react-admin'
// eslint-disable-next-line import/no-extraneous-dependencies
import { useQuery } from 'react-query'

import type {
  BasicKojiEntry,
  Feature,
  FeatureCollection,
} from '@assets/types'
import { fetchWrapper } from '@services/fetches'
import { useImportExport } from '@hooks/useImportExport'

export function BaseButton({
  onClick,
}: {
  onClick: React.MouseEventHandler<HTMLButtonElement> | undefined
}) {
  return (
    <Button label="Export" size="small" onClick={onClick}>
      <ExportIcon />
    </Button>
  )
}

export function getUrl(resource: string, id: number) {
  // v2: `GET /api/v2/{geofences|routes}/{id}` returns the feature(s) for export.
  // `project` exported a project's geofences as a FeatureCollection in v1 — there
  // is no v2 project→FeatureCollection export, so fall back to the geofence by id
  // (best-effort).
  // TODO(v2-gap): the v1 `project/feature-collection/{id}` (all of a project's
  // geofences in one FC) export has no v2 equivalent; project export here returns
  // a single geofence by the project id, which is almost certainly wrong. Flagged.
  const seg =
    resource === 'project'
      ? 'geofences'
      : resource === 'geofence'
      ? 'geofences'
      : resource === 'route'
      ? 'routes'
      : resource
  return `/api/v2/${seg}/${id}`
}

export function ExportButton<T extends BasicKojiEntry>({
  resource,
}: {
  resource: string
}) {
  const record = useRecordContext<T>()
  const { refetch } = useQuery(
    `export-${resource}-${record.id}`,
    () =>
      fetchWrapper<Feature | FeatureCollection>(getUrl(resource, record.id)),
    {
      enabled: false,
    },
  )

  return (
    <BaseButton
      onClick={(event) => {
        event.stopPropagation()
        refetch().then((res) => {
          // `res.data` is the unwrapped v2 payload (Feature | FeatureCollection).
          if (res?.data) {
            useImportExport.setState({
              open: 'exportPolygon',
              feature: res.data,
              fileName:
                res.data.type === 'Feature'
                  ? res.data.properties?.__name ||
                    res.data.properties?.name ||
                    ''
                  : record.name,
            })
          }
        })
      }}
    />
  )
}

export function BulkExportButton<T extends BasicKojiEntry>({
  resource,
}: {
  resource: string
}) {
  const { selectedIds } = useListContext<T>()
  const unselectAll = useUnselectAll(resource)
  const { refetch } = useQuery(
    `export-${resource}`,
    () =>
      Promise.all(
        selectedIds.map((id) => fetchWrapper<Feature>(getUrl(resource, id))),
      ),
    {
      enabled: false,
    },
  )

  return (
    <BaseButton
      onClick={(event) => {
        event.stopPropagation()
        unselectAll()
        refetch().then((res) => {
          // Each entry is the unwrapped v2 Feature payload (or null on miss).
          if (res?.data) {
            useImportExport.setState({
              open: 'exportPolygon',
              feature: {
                type: 'FeatureCollection',
                features: res.data.filter((r): r is Feature => !!r),
              },
            })
          }
        })
      }}
    />
  )
}
