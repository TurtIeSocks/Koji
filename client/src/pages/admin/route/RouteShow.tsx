import * as React from 'react'
import {
  FunctionField,
  ReferenceField,
  Show,
  SimpleShowLayout,
  TextField,
} from 'react-admin'
import { Typography } from '@mui/material'
import { KojiRoute } from '@assets/types'
import { Code } from '@components/Code'

export default function GeofenceShow() {
  return (
    <Show>
      <SimpleShowLayout>
        <Typography variant="h6" gutterBottom>
          Overview
        </Typography>
        <TextField source="name" />
        <TextField source="description" />
        <TextField source="mode" />
        <ReferenceField source="geofence_id" reference="geofence" />
        <FunctionField<KojiRoute>
          label="Geometry"
          render={(fence) => {
            const parsed: string =
              typeof fence?.geometry === 'string'
                ? JSON.stringify(JSON.parse(fence?.geometry), null, 2)
                : JSON.stringify(fence?.geometry, null, 2)
            return <Code maxHeight="50vh">{parsed}</Code>
          }}
        />
      </SimpleShowLayout>
    </Show>
  )
}
