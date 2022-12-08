import * as React from 'react'
import { Button, Dialog, DialogActions } from '@mui/material'
import type { FeatureCollection } from 'geojson'

import DialogHeader from './Header'

interface Props {
  open: string
  setOpen: (open: string) => void
  geojson: FeatureCollection
}

export default function SaveToKoji({ open, setOpen, geojson }: Props) {
  const [remote, setRemote] = React.useState<FeatureCollection>(geojson)

  React.useEffect(() => {
    fetch('/api/v1/geofence/all').then(async (res) =>
      console.log(await res.json()),
    )
  }, [])

  return (
    <Dialog open={open === 'save_koji'} onClose={() => setOpen('')}>
      <DialogHeader action={() => setOpen('')}>Save to Kōji</DialogHeader>
      <DialogActions>
        <Button
          onClick={() => {
            fetch('/api/v1/geofence/save', {
              method: 'POST',
              headers: {
                'Content-Type': 'application/json',
              },
              body: JSON.stringify({ area: geojson }),
            })
              .then(async (res) => console.log(await res.json()))
              .catch((err) => console.log(err))
            // setOpen('')
          }}
        >
          Save
        </Button>
        <Button
          onClick={() => {
            setOpen('')
          }}
        >
          Close
        </Button>
      </DialogActions>
    </Dialog>
  )
}
