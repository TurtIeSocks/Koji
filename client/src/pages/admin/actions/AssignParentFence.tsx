/* eslint-disable import/no-extraneous-dependencies */
import * as React from 'react'
import { Button, useListContext, useNotify } from 'react-admin'
import { useMutation } from 'react-query'
import { fetchUtils, useGetMany, useRefresh, useUnselectAll } from 'ra-core'
import {
  Dialog,
  DialogActions,
  DialogContent,
  MenuItem,
  Select,
} from '@mui/material'

import DialogHeader from '@components/dialogs/Header'
import { useRaStore } from '@hooks/useRaStore'
import Grid2 from '@mui/material/Unstable_Grid2/Grid2'

export function AssignParentToFences({ open }: { open: boolean }) {
  const { selectedIds } = useListContext()
  const unSelectAll = useUnselectAll('geofence')
  const notify = useNotify()
  const refresh = useRefresh()

  const { data } = useGetMany('geofence', {
    ids: [0],
  })

  const options: Record<string, number> = Object.fromEntries(
    data?.map((x) => [x.name, x.id]) ?? [],
  )
  const setRaStore = useRaStore((s) => s.setRaStore)

  const [selected, setSelected] = React.useState(0)

  const assignProjectsToFence = useMutation(
    () => {
      return Promise.all(
        selectedIds.map((id) =>
          fetchUtils.fetchJson(
            `/internal/admin/assign/geofence/parent/${id}/`,
            {
              method: 'PATCH',
              body: JSON.stringify(selected),
            },
          ),
        ),
      )
    },
    {
      onSuccess: () => {
        refresh()
        notify(
          `Successfully assigned ${
            data?.find((d) => d.id === selected)?.name || ''
          } to ${selectedIds.length} geofence(s)`,
          {
            type: 'success',
          },
        )
      },
      onError: () => {
        refresh()
        notify(
          `Failed to assign ${
            data?.find((d) => d.id === selected)?.name || ''
          } to ${selectedIds.length} geofence(s)`,
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
      onClose={() => setRaStore('bulkAssignParent', false)}
      maxWidth="xl"
    >
      <DialogHeader>Assign Parent to Selected Geofence(s)</DialogHeader>
      <DialogContent>
        <Grid2 container minHeight="20vh">
          <Grid2 xs={10}>
            <Select
              fullWidth
              value={selected}
              onChange={(e) => setSelected(+e.target.value)}
            >
              {Object.entries(options).map(([name, id]) => (
                <MenuItem key={id} value={id}>
                  {name}
                </MenuItem>
              ))}
            </Select>
          </Grid2>
        </Grid2>
      </DialogContent>
      <DialogActions>
        <Button
          label="Close"
          color="secondary"
          onClick={() => setRaStore('bulkAssignParent', false)}
        />
        <Button
          label="Save"
          color="primary"
          onClick={(event) => {
            event.stopPropagation()
            assignProjectsToFence.mutate()
            unSelectAll()
            setRaStore('bulkAssignParent', false)
          }}
        />
      </DialogActions>
    </Dialog>
  )
}

export function BulkAssignFenceButton() {
  const setRaStore = useRaStore((s) => s.setRaStore)
  const open = useRaStore((s) => s.bulkAssignParent)

  return (
    <>
      <Button
        label="Assign Parent"
        onClick={() => {
          setRaStore('bulkAssignParent', true)
        }}
      />
      <AssignParentToFences open={open} />
    </>
  )
}
