import * as React from 'react'
import { Show, SimpleShowLayout, TextField } from 'react-admin'
import { Typography } from '@mui/material'

export default function TileServerShow() {
  return (
    <Show>
      <SimpleShowLayout>
        <Typography variant="h6" gutterBottom>
          Overview
        </Typography>
        <TextField source="name" />
        <TextField source="url" />
      </SimpleShowLayout>
    </Show>
  )
}
