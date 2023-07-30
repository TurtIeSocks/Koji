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

import { BasicKojiEntry, Feature, KojiResponse } from '@assets/types'
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
  return `/api/v1/${resource === 'project' ? 'geofence' : resource}/${
    resource === 'project' ? 'feature-collection' : 'area'
  }/${id}?&name=true&parent=true&mode=true`
}

export function ExportButton<T extends BasicKojiEntry>({
  resource,
}: {
  resource: string
}) {
  const record = useRecordContext<T>()
  const { refetch } = useQuery(
    `export-${resource}-${record.id}`,
    () => fetchWrapper<KojiResponse<Feature>>(getUrl(resource, record.id)),
    {
      enabled: false,
    },
  )

  return (
    <BaseButton
      onClick={(event) => {
        event.stopPropagation()
        refetch().then((res) => {
          if (res?.data) {
            useImportExport.setState({
              open: 'exportPolygon',
              feature: res.data.data,
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
        selectedIds.map((id) =>
          fetchWrapper<KojiResponse<Feature>>(getUrl(resource, id)),
        ),
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
          if (res?.data) {
            useImportExport.setState({
              open: 'exportPolygon',
              feature: {
                type: 'FeatureCollection',
                features: res.data
                  .filter((r) => r?.data)
                  .map((d) => d?.data as Feature),
              },
            })
          }
        })
      }}
    />
  )
}
