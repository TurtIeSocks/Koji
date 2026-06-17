import * as React from 'react'
import { DateField, Show, SimpleShowLayout, TextField } from 'react-admin'
import { Typography } from '@mui/material'

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
