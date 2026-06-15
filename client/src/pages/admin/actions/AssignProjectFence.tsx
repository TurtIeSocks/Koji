/* eslint-disable import/no-extraneous-dependencies */
import * as React from 'react'
import { Button, useListContext, useNotify } from 'react-admin'
import { useMutation } from 'react-query'
import { fetchUtils, useGetMany, useRefresh, useUnselectAll } from 'ra-core'
import {
  Dialog,
  DialogActions,
  DialogContent,
  Typography,
  capitalize,
} from '@mui/material'

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
      // v2: the geofence↔project link lives on the GEOFENCE side
      // (`PATCH /api/v2/geofences/{id} { projects }`).
      // - resource === 'geofence': each selected geofence id gets the chosen
      //   project ids.
      // - resource === 'project': assign the (single) selected project to the
      //   chosen geofences — so loop the geofence ids, each PATCHed with this
      //   project id.
      // TODO(v2-verify): the v1 `/internal/admin/geofence_project/{resource}/{id}`
      // did a full replace of the link set (unassigning non-selected). The v2
      // PATCH here sets `projects` to exactly the chosen ids for geofence-mode;
      // for project-mode it replaces each fence's project list with just this
      // project (dropping that fence's OTHER projects). Reconcile against a live
      // deploy — a dedicated link endpoint may be needed for non-destructive
      // multi-project assignment.
      const projectIds = selected.map((x) => x.id)
      if (resource === 'geofence') {
        return Promise.all(
          selectedIds.map((id) =>
            fetchUtils.fetchJson(`/api/v2/geofences/${id}`, {
              method: 'PATCH',
              body: JSON.stringify({ projects: projectIds }),
            }),
          ),
        )
      }
      // resource === 'project': `selectedIds` are project ids, `selected` are the
      // chosen geofences. Assign each chosen geofence to these project ids.
      return Promise.all(
        selected.map((fence) =>
          fetchUtils.fetchJson(`/api/v2/geofences/${fence.id}`, {
            method: 'PATCH',
            body: JSON.stringify({ projects: selectedIds }),
          }),
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

  const reset = () => {
    setSelected([])
    unSelectAll()
    setRaStore(storeKey, false)
  }

  const opposite = resource === 'project' ? 'geofence' : 'project'

  return (
    <Dialog open={open} onClose={reset} maxWidth="sm">
      <DialogHeader>
        Assign {capitalize(opposite)}(s) to selected {capitalize(resource)}(s)
      </DialogHeader>
      <DialogContent sx={{ my: 3 }}>
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
              label={`Select ${capitalize(opposite)}s`}
            />
          </Grid2>
          <Grid2 xs={12}>
            <Typography variant="h6" p={4} pb={0}>
              Choose which {opposite}s you would like to assign to the{' '}
              {selectedIds.length} selected {resource}s. This will unassign all
              non-selected {opposite}s from those {resource}s, if any.
            </Typography>
          </Grid2>
        </Grid2>
      </DialogContent>
      <DialogActions>
        <Button label="Close" color="secondary" onClick={reset} />
        <Button
          label="Save"
          color="primary"
          onClick={(event) => {
            event.stopPropagation()
            assignProjectsToFence.mutate()
            reset()
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
