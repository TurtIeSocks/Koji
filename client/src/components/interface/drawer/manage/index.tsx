import * as React from 'react'
import {
  Divider,
  List,
  ListItemButton,
  ListItemIcon,
  ListSubheader,
} from '@mui/material'
import { Download, Save, Upload } from '@mui/icons-material'

import { useStatic } from '@hooks/useStatic'

import SaveToKoji from '@components/interface/dialogs/Manager'
import ExportRoute from '../../dialogs/ExportRoute'
import PolygonDialog from '../../dialogs/Polygon'
import InstanceSelect from './Instance'

export default function ImportExport() {
  const [open, setOpen] = React.useState('')
  const geojson = useStatic((s) => s.geojson)

  return (
    <List dense>
      <ListSubheader disableGutters>Import from Scanner</ListSubheader>
      <InstanceSelect endpoint="/api/instance/all" stateKey="instances" />
      <ListSubheader disableGutters>Import from Kōji</ListSubheader>
      <InstanceSelect endpoint="/api/v1/geofence/all" stateKey="geofences" />
      <Divider sx={{ my: 2 }} />
      <ListItemButton onClick={() => setOpen('polygon')}>
        <ListItemIcon>
          <Download />
        </ListItemIcon>
        Import Polygon
      </ListItemButton>
      <ListItemButton onClick={() => setOpen('route')}>
        <ListItemIcon>
          <Upload />
        </ListItemIcon>
        Export Route
      </ListItemButton>
      <ListItemButton onClick={() => setOpen('manager')}>
        <ListItemIcon>
          <Save />
        </ListItemIcon>
        Open Manager
      </ListItemButton>
      <PolygonDialog
        mode="import"
        open={open}
        setOpen={setOpen}
        feature={geojson}
      />
      <ExportRoute open={open} setOpen={setOpen} />
      <SaveToKoji open={open} setOpen={setOpen} geojson={geojson} />
    </List>
  )
}
