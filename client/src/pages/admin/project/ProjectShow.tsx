import * as React from 'react'
import {
  BooleanField,
  ChipField,
  ReferenceArrayField,
  Show,
  SimpleShowLayout,
  SingleFieldList,
  TextField,
} from 'react-admin'
import { Typography } from '@mui/material'

export default function ProjectShow() {
  return (
    <Show>
      <SimpleShowLayout>
        <Typography variant="h6" gutterBottom>
          Overview
        </Typography>
        <TextField source="name" />
        <TextField source="api_endpoint" />
        <TextField source="api_key" />
        <BooleanField source="scanner" />
        <ReferenceArrayField
          label="Geofences"
          source="geofences"
          reference="geofence"
        >
          <SingleFieldList>
            <ChipField source="name" />
          </SingleFieldList>
        </ReferenceArrayField>
      </SimpleShowLayout>
    </Show>
  )
}
