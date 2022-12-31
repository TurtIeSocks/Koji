import * as React from 'react'
import {
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  Link,
} from '@mui/material'
import { useRaStore } from '@hooks/useRaStore'
import Add from '@mui/icons-material/Add'

export function CreateDialog() {
  const geofenceCreateDialog = useRaStore((state) => state.geofenceCreateDialog)
  const setRaStore = useRaStore((state) => state.setRaStore)

  return (
    <Dialog
      open={geofenceCreateDialog}
      onClose={() => setRaStore('geofenceCreateDialog', false)}
    >
      <DialogContent>
        Creating a geofence from the admin panel is currently not supported. To
        create one or import from a file, please go to the{' '}
        <Link href="/map">map</Link> and open the &quot;JSON Manager&quot; from
        the side panel.
      </DialogContent>
      <DialogActions>
        <Button onClick={() => setRaStore('geofenceCreateDialog', false)}>
          Close
        </Button>
      </DialogActions>
    </Dialog>
  )
}

export default function GeofenceCreateButton() {
  const setRaStore = useRaStore((state) => state.setRaStore)

  return (
    <>
      <Button
        color="primary"
        onClick={() => setRaStore('geofenceCreateDialog', true)}
      >
        <Add />
        Create
      </Button>
      <CreateDialog />
    </>
  )
}
