import * as React from 'react'
import {
  ChipField,
  DateField,
  FunctionField,
  ReferenceArrayField,
  Show,
  SimpleShowLayout,
  SingleFieldList,
  TextField,
  useRecordContext,
} from 'react-admin'
import { ListItemText, Typography } from '@mui/material'
import { KojiGeofence, KojiProperty } from '@assets/types'
import { Code } from '@components/Code'

export default function PropertyShow() {
  return (
    <Show>
      <SimpleShowLayout>
        <Typography variant="h6" gutterBottom>
          Overview
        </Typography>
        <TextField source="name" />
        <TextField source="category" />
        <TextField source="default_value" label="Default Value" />
        <DateField source="created_at" />
        <DateField source="updated_at" />
      </SimpleShowLayout>
    </Show>
  )
}
