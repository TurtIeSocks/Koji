import * as React from 'react'
import SyncIcon from '@mui/icons-material/Sync'
import {
  Button,
  useListContext,
  useNotify,
  useRecordContext,
  useUnselectAll,
} from 'react-admin'
// eslint-disable-next-line import/no-extraneous-dependencies
import { useMutation } from 'react-query'

import type { BasicKojiEntry } from '@assets/types'
import { capitalize } from '@mui/material'
import { fetchWrapper } from '@services/fetches'

export function BaseButton({
  onClick,
}: {
  onClick: React.MouseEventHandler<HTMLButtonElement> | undefined
}) {
  return (
    <Button label="Sync" size="small" onClick={onClick}>
      <SyncIcon />
    </Button>
  )
}

export function PushToProd<T extends BasicKojiEntry>({
  resource,
}: {
  resource: string
}) {
  const record = useRecordContext<T>()
  const notify = useNotify()

  const sync = useMutation(
    () => fetchWrapper(`/api/v1/${resource}/push/${record.id}`),
    {
      onSuccess: () => {
        notify(`${record.name} synced with scanner`, {
          type: 'success',
        })
      },
      onError: () => {
        notify(`Failed to sync ${record.name}`, {
          type: 'error',
        })
      },
    },
  )

  return (
    <BaseButton
      onClick={(event) => {
        event.stopPropagation()
        sync.mutate()
      }}
    />
  )
}

export function BulkPushToProd<T extends BasicKojiEntry>({
  resource,
}: {
  resource: string
}) {
  const { selectedIds } = useListContext<T>()
  const unselectAll = useUnselectAll(resource)
  const notify = useNotify()

  const sync = useMutation(
    () =>
      Promise.all(
        selectedIds.map((id) => fetchWrapper(`/api/v1/${resource}/push/${id}`)),
      ),
    {
      onSuccess: () => {
        notify(
          `${selectedIds.length} ${capitalize(resource)}${
            selectedIds.length > 1 ? 's' : ''
          } synced with scanner`,
          {
            type: 'success',
          },
        )
      },
      onError: () => {
        notify(`Failed to start quest on ${selectedIds.length} area(s)`, {
          type: 'error',
        })
      },
    },
  )

  return (
    <BaseButton
      onClick={(event) => {
        event.stopPropagation()
        unselectAll()
        sync.mutate()
      }}
    />
  )
}
