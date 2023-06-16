import * as React from 'react'
import {
  ChipField,
  FunctionField,
  ReferenceArrayField,
  Show,
  SimpleShowLayout,
  SingleFieldList,
  TextField,
  useRecordContext,
} from 'react-admin'
import { ListItemText, Typography } from '@mui/material'
import { AdminGeofence, KojiGeofence } from '@assets/types'
import { Code } from '@components/Code'
import { GeofenceMap } from './GeofenceMap'

function PropertyFields() {
  const record = useRecordContext<AdminGeofence>()
  return (
    <>
      <ListItemText secondary="Properties" />
      {record.properties.map((entry) => (
        <ListItemText
          key={entry.id}
          primary={`${entry.name}: ${entry.value}`}
        />
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
        <TextField source="mode" />
        <TextField source="geo_type" />
        <TextField source="area.geometry.type" label="Geometry Type" />
        <PropertyFields />
        <ReferenceArrayField
          label="Projects"
          source="projects"
          reference="project"
        >
          <SingleFieldList>
            <ChipField source="name" />
          </SingleFieldList>
        </ReferenceArrayField>
        <FunctionField
          label="Preview"
          render={(fence: AdminGeofence) =>
            fence ? <GeofenceMap formData={fence} /> : null
          }
        />
        <FunctionField
          label="Geometry"
          render={(fence: KojiGeofence) => {
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
