import * as React from 'react'
import { useRecordContext } from 'react-admin'
import { AdminGeofence } from '@assets/types'
import { ListItemText } from '@mui/material'
import Grid2 from '@mui/material/Unstable_Grid2/Grid2'

export function GeofenceExpand() {
  const record = useRecordContext<AdminGeofence>()
  return (
    <Grid2 container justifyContent="flex-start">
      <Grid2 xs={4}>
        <ListItemText
          primary="Projects"
          secondary={record.projects?.length || 0}
        />
      </Grid2>
      <Grid2 xs={4}>
        <ListItemText
          primary="Properties"
          secondary={record.properties?.length || 0}
        />
      </Grid2>
      <Grid2 xs={4}>
        <ListItemText primary="Routes" secondary={record.routes?.length || 0} />
      </Grid2>

      {(record.properties || []).map((p) => (
        <Grid2 key={p.name} xs={6} sm={4} md={3} lg={2}>
          <ListItemText primary={p.name} secondary={`${p.value}`} />
        </Grid2>
      ))}
    </Grid2>
  )
}
