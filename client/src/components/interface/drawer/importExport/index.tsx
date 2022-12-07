import * as React from 'react'
import { Divider, List, ListItemButton, ListItemIcon } from '@mui/material'
import { ContentCopy, Save } from '@mui/icons-material'

import { useStatic } from '@hooks/useStatic'

import ExportRoute from '../../dialogs/ExportRoute'
import PolygonDialog from '../../dialogs/Polygon'
import InstanceSelect from './Instance'

export default function ImportExport() {
  const [open, setOpen] = React.useState('')

  const geojson = useStatic((s) => s.geojson)

  return (
    <List dense>
      <InstanceSelect />
      <Divider sx={{ my: 2 }} />
      <ListItemButton onClick={() => setOpen('polygon')}>
        <ListItemIcon>
          <ContentCopy />
        </ListItemIcon>
        Import Polygon
      </ListItemButton>
      <ListItemButton onClick={() => setOpen('route')}>
        <ListItemIcon>
          <ContentCopy />
        </ListItemIcon>
        Export Route
      </ListItemButton>
      <ListItemButton onClick={() => setOpen('polygon')}>
        <ListItemIcon>
          <Save />
        </ListItemIcon>
        Save to Kōji
      </ListItemButton>
      <PolygonDialog
        mode="import"
        open={open}
        setOpen={setOpen}
        feature={geojson}
      />
      <ExportRoute open={open} setOpen={setOpen} />
    </List>
  )
}
