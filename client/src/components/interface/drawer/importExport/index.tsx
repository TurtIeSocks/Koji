import * as React from 'react'
import { List, ListItemButton, ListItemIcon } from '@mui/material'
import { ContentCopy } from '@mui/icons-material'

import { useStatic } from '@hooks/useStatic'

import ExportRoute from '../../dialogs/ExportRoute'
import PolygonDialog from '../../dialogs/Polygon'

export default function ImportExport() {
  const [open, setOpen] = React.useState('')

  const geojson = useStatic((s) => s.geojson)

  return (
    <>
      <List dense>
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
      </List>

      <PolygonDialog
        mode="import"
        open={open}
        setOpen={setOpen}
        feature={geojson}
      />
      <ExportRoute open={open} setOpen={setOpen} />
    </>
  )
}
