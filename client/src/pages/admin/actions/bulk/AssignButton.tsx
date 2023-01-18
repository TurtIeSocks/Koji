/* eslint-disable import/no-extraneous-dependencies */
import * as React from 'react'
import { Button, useListContext, useNotify } from 'react-admin'
import { useMutation } from 'react-query'
import { fetchUtils, useGetMany, useRefresh, useUnselectAll } from 'ra-core'
import { Dialog, DialogActions, DialogContent, capitalize } from '@mui/material'

import DialogHeader from '@components/dialogs/Header'
import { useRaStore } from '@hooks/useRaStore'
import KojiAuto from '@components/AutoComplete'
import Grid2 from '@mui/material/Unstable_Grid2/Grid2'

interface Props {
  resource: string
  storeKey: 'bulkAssignGeofence' | 'bulkAssignProject'
  open: boolean
}

export function AssignFencesToProjects({ resource, storeKey, open }: Props) {
  const { selectedIds } = useListContext()
  const unSelectAll = useUnselectAll(resource)
  const notify = useNotify()
  const refresh = useRefresh()

  const { data, isLoading } = useGetMany(
    resource === 'project' ? 'geofence' : 'project',
    {
      ids: [0],
    },
  )
  const options: Record<string, number> = Object.fromEntries(
    data?.map((x) => [x.name, x.id]) ?? [],
  )
  const setRaStore = useRaStore((s) => s.setRaStore)

  const [selected, setSelected] = React.useState<
    { id: number; name: string }[]
  >([])

  const assignProjectsToFence = useMutation(
    () => {
      return Promise.all(
        selectedIds.map((id) =>
          fetchUtils.fetchJson(
            `/internal/admin/geofence_project/${resource}/${id}/`,
            {
              method: 'PATCH',
              body: JSON.stringify(selected.map((x) => x.id)),
            },
          ),
        ),
      )
    },
    {
      onSuccess: () => {
        refresh()
        notify(
          `${selected.length} ${
            resource === 'project' ? 'geofences' : 'projects'
          }(s) assigned to ${selectedIds.length} ${resource}(s)`,
          {
            type: 'success',
          },
        )
      },
      onError: () => {
        refresh()
        notify(
          `Failed to ${selected.length} ${
            resource === 'project' ? 'geofences' : 'projects'
          }(s) assign to ${selectedIds.length} ${resource}(s)`,
          {
            type: 'error',
          },
        )
      },
    },
  )

  return (
    <Dialog
      open={open}
      onClose={() => setRaStore(storeKey, false)}
      maxWidth="xl"
    >
      <DialogHeader>
        Assign {capitalize(resource)} to Selected{' '}
        {resource === 'project' ? 'Project' : 'Geofence'}(s)
      </DialogHeader>
      <DialogContent>
        <Grid2 container minHeight="20vh">
          <Grid2 xs={12}>
            <KojiAuto
              selected={selected.map((x) => x.name)}
              onChange={(_e, newValues) => {
                setSelected(
                  newValues.map((name) => ({ id: options[name], name })),
                )
              }}
              options={options}
              loading={isLoading}
              label={`Select ${
                resource === 'project' ? 'Geofence' : 'Project'
              }s`}
            />
          </Grid2>
        </Grid2>
      </DialogContent>
      <DialogActions>
        <Button
          label="Close"
          color="secondary"
          onClick={() => setRaStore(storeKey, false)}
        />
        <Button
          label="Save"
          color="primary"
          onClick={(event) => {
            event.stopPropagation()
            assignProjectsToFence.mutate()
            unSelectAll()
            setRaStore(storeKey, false)
          }}
        />
      </DialogActions>
    </Dialog>
  )
}

export function BulkAssignButton({ resource }: { resource: string }) {
  const storeKey =
    resource === 'project' ? 'bulkAssignGeofence' : 'bulkAssignProject'
  const setRaStore = useRaStore((s) => s.setRaStore)
  const open = useRaStore((s) => s[storeKey])

  return (
    <>
      <Button
        label={`Assign ${capitalize(
          resource === 'project' ? 'geofence' : 'project',
        )}`}
        onClick={() => {
          setRaStore(storeKey, true)
        }}
      />
      <AssignFencesToProjects
        resource={resource}
        storeKey={storeKey}
        open={open}
      />
    </>
  )
}
