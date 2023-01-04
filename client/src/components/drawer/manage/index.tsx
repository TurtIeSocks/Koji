import * as React from 'react'
import {
  Divider,
  List,
  ListItemButton,
  ListItemIcon,
  ListItemText,
  ListSubheader,
} from '@mui/material'
import Code from '@mui/icons-material/Code'
import Save from '@mui/icons-material/Save'
import Upload from '@mui/icons-material/Upload'

import { useStatic } from '@hooks/useStatic'

import RawManager from '@components/dialogs/Manager'
import ExportRoute from '../../dialogs/ExportRoute'
import PolygonDialog from '../../dialogs/Polygon'
import InstanceSelect from './Instance'
import ShapeFile from './ShapeFile'
import JsonFile from './Json'

export default function ImportExport() {
  const [open, setOpen] = React.useState('')
  const [exportAll, setExportAll] = React.useState(false)
  const geojson = useStatic((s) => s.geojson)

  return (
    <List dense>
      <ListSubheader disableGutters>Import from Scanner</ListSubheader>
      <InstanceSelect endpoint="/internal/routes/from_scanner" />
      <ListSubheader disableGutters>Import from Kōji</ListSubheader>
      <InstanceSelect endpoint="/internal/routes/from_koji" koji />
      <ListSubheader disableGutters>Additional Importing Methods</ListSubheader>
      <ListItemButton onClick={() => setOpen('polygon')}>
        <ListItemIcon>
          <Code />
        </ListItemIcon>
        <ListItemText primary="Geofence Input" />
      </ListItemButton>
      <JsonFile />
      <ShapeFile />
      <ListSubheader disableGutters>Exporting</ListSubheader>
      <ListItemButton
        onClick={() => {
          setOpen('polygon')
          setExportAll(true)
        }}
      >
        <ListItemIcon>
          <Upload />
        </ListItemIcon>
        Export All Polygons
      </ListItemButton>
      <ListItemButton onClick={() => setOpen('route')}>
        <ListItemIcon>
          <Upload />
        </ListItemIcon>
        Export All Routes
      </ListItemButton>
      <Divider sx={{ my: 2 }} />
      <ListItemButton onClick={() => setOpen('rawManager')}>
        <ListItemIcon>
          <Save />
        </ListItemIcon>
        <ListItemText primary="JSON Manager" />
      </ListItemButton>
      <PolygonDialog
        mode={exportAll ? 'exportAll' : 'import'}
        open={open}
        setOpen={setOpen}
        feature={{
          ...geojson,
          features: geojson.features.filter(
            (feat) =>
              feat.geometry.type === 'Polygon' ||
              feat.geometry.type === 'MultiPolygon',
          ),
        }}
      />
      <ExportRoute
        open={open}
        setOpen={setOpen}
        geojson={{
          ...geojson,
          features: geojson.features.filter(
            (feat) =>
              feat.geometry.type === 'Point' ||
              feat.geometry.type === 'MultiPoint',
          ),
        }}
      />
      <RawManager open={open} setOpen={setOpen} geojson={geojson} />
    </List>
  )
}
