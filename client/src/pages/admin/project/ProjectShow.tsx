import * as React from 'react'
import {
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
        <ReferenceArrayField
          label="Geofences"
          source="related"
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
