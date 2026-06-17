import * as React from 'react'
import { FunctionField, Show, SimpleShowLayout, TextField } from 'react-admin'
import { Typography } from '@mui/material'
import { KojiTileServer } from '@assets/types'
import TileServerMap from './TileServerMap'

export default function TileServerShow() {
  return (
    <Show>
      <SimpleShowLayout>
        <Typography variant="h6" gutterBottom>
          Overview
        </Typography>
        <TextField source="name" />
        <TextField source="url" />
        <FunctionField
          label="Preview"
          render={(tileServer: KojiTileServer) =>
            tileServer ? <TileServerMap formData={tileServer} /> : null
          }
        />
      </SimpleShowLayout>
    </Show>
  )
}
