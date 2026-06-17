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
import RouteMap from './RouteMap'

export default function RouteShow() {
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
        <TextField source="points" />
        <FunctionField
          label="Preview"
          render={(route: KojiRoute) =>
            route ? <RouteMap formData={route} /> : null
          }
        />
        <FunctionField
          label="Geometry"
          render={(route: KojiRoute) => {
            const parsed: string =
              typeof route?.geometry === 'string'
                ? JSON.stringify(JSON.parse(route?.geometry), null, 2)
                : JSON.stringify(route?.geometry, null, 2)
            return <Code maxHeight="50vh">{parsed}</Code>
          }}
        />
      </SimpleShowLayout>
    </Show>
  )
}
