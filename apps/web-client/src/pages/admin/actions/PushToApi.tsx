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
import { SxProps, capitalize } from '@mui/material'
import { fetchWrapper } from '@services/fetches'

// react-admin resource name → v2 segment for the publish action.
// TODO(v2-verify): the v1 GET `/{resource}/push/{id}` (sync golbat-sync) maps to
// the v2 `POST /api/v2/{seg}/{id}/publish` (async Dragonite-area publish via the
// event outbox). The semantics differ (publish vs golbat-sync) and a geofence
// with no linked Dragonite area returns 422 — confirm the intended behavior +
// success/error UX against a live deploy.
const PUBLISH_SEG: Record<string, string> = {
  geofence: 'geofences',
  route: 'routes',
}
const publishSeg = (resource: string): string =>
  PUBLISH_SEG[resource] ?? resource

export function BaseButton({
  onClick,
  sx,
}: {
  onClick: React.MouseEventHandler<HTMLButtonElement> | undefined
  sx?: SxProps
}) {
  return (
    <Button label="Sync" size="small" onClick={onClick} sx={sx}>
      <SyncIcon />
    </Button>
  )
}

export function PushToProd<T extends BasicKojiEntry>({
  resource,
  sx,
}: {
  resource: string
  sx?: SxProps
}) {
  const record = useRecordContext<T>()
  const notify = useNotify()

  const sync = useMutation(
    () =>
      fetchWrapper(`/api/v2/${publishSeg(resource)}/${record.id}/publish`, {
        method: 'POST',
      }),
    {
      onSuccess: () => {
        notify(`${record.name} synced with golbat`, {
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
      sx={sx}
      onClick={(event) => {
        event.stopPropagation()
        sync.mutate()
      }}
    />
  )
}

export function BulkPushToProd<T extends BasicKojiEntry>({
  resource,
  sx,
}: {
  resource: string
  sx?: SxProps
}) {
  const { selectedIds } = useListContext<T>()
  const unselectAll = useUnselectAll(resource)
  const notify = useNotify()

  const sync = useMutation(
    () =>
      Promise.all(
        selectedIds.map((id) =>
          fetchWrapper(`/api/v2/${publishSeg(resource)}/${id}/publish`, {
            method: 'POST',
          }),
        ),
      ),
    {
      onSuccess: () => {
        notify(
          `${selectedIds.length} ${capitalize(resource)}${
            selectedIds.length > 1 ? 's' : ''
          } synced with golbat`,
          {
            type: 'success',
          },
        )
      },
      onError: () => {
        notify(`Failed to sync ${selectedIds.length} area(s)`, {
          type: 'error',
        })
      },
    },
  )

  return (
    <BaseButton
      sx={sx}
      onClick={(event) => {
        event.stopPropagation()
        unselectAll()
        sync.mutate()
      }}
    />
  )
}
