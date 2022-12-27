import * as React from 'react'
import {
  FunctionField,
  Show,
  SimpleShowLayout,
  TextField,
  useRecordContext,
} from 'react-admin'
import { ListItemText, Typography } from '@mui/material'
import { KojiGeofence } from '@assets/types'
import { Code } from '@components/Code'

function PropertyFields() {
  const record = useRecordContext<KojiGeofence>()
  return (
    <>
      <ListItemText secondary="Properties" />
      {Object.entries(record.area?.properties || {}).map(([k, v]) => (
        <ListItemText key={k} primary={`${k}: ${v}`} />
      ))}
    </>
  )
}

export default function GeofenceShow() {
  return (
    <Show>
      <SimpleShowLayout>
        <Typography variant="h6" gutterBottom>
          Overview
        </Typography>
        <TextField source="name" />
        <TextField source="area.geometry.type" label="Geometry Type" />
        <PropertyFields />
        <FunctionField<KojiGeofence>
          label="Area"
          render={(fence) => {
            const parsed: string =
              typeof fence?.area === 'string'
                ? fence?.area
                : JSON.stringify(fence?.area, null, 2)
            return <Code maxHeight="50vh">{parsed}</Code>
          }}
        />
      </SimpleShowLayout>
    </Show>
  )
}
