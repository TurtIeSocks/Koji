import * as React from 'react'
import {
  List,
  ListItemButton,
  ListItemIcon,
  ListItemText,
  ListSubheader,
} from '@mui/material'
import Save from '@mui/icons-material/Save'
import Upload from '@mui/icons-material/Upload'
import AutoFix from '@mui/icons-material/AutoFixHigh'
import Code from '@mui/icons-material/Code'

import { useStatic } from '@hooks/useStatic'
import RawManager from '@components/dialogs/Manager'
import ImportWizard from '@components/dialogs/import/ImportWizard'

import ExportRoute from '../../dialogs/ExportRoute'
import PolygonDialog from '../../dialogs/Polygon'
import InstanceSelect from './Instance'

export default function ImportExport() {
  const [open, setOpen] = React.useState('')
  const [exportAll, setExportAll] = React.useState(false)
  const geojson = useStatic((s) => s.geojson)
  const setStatic = useStatic((s) => s.setStatic)

  return (
    <List dense>
      <ListSubheader disableGutters>Import from Scanner</ListSubheader>
      <InstanceSelect
        endpoint="/internal/routes/from_scanner"
        controlled
        initialState={geojson.features
          .filter(
            (feat) =>
              feat.geometry.type !== 'LineString' &&
              feat.geometry.type !== 'Point' &&
              typeof feat.id === 'string' &&
              feat.id.endsWith('__SCANNER'),
          )
          .map(
            (feat) =>
              `${feat.properties?.__name}__${feat.properties?.__type || ''}`,
          )}
      />
      <ListSubheader disableGutters>Import from Kōji</ListSubheader>
      <InstanceSelect
        endpoint="/internal/routes/from_koji"
        koji
        controlled
        initialState={geojson.features
          .filter(
            (feat) =>
              feat.geometry.type !== 'LineString' &&
              feat.geometry.type !== 'Point' &&
              typeof feat.id === 'string' &&
              feat.id.endsWith('__KOJI'),
          )
          .map(
            (feat) =>
              `${feat.properties?.__name}__${feat.properties?.__type || ''}`,
          )}
      />
      <ListSubheader disableGutters>Manual Importing</ListSubheader>
      <ListItemButton
        onClick={() =>
          setStatic('importWizard', (prev) => ({ ...prev, open: true }))
        }
      >
        <ListItemIcon>
          <AutoFix />
        </ListItemIcon>
        <ListItemText primary="Import Wizard" />
      </ListItemButton>
      <ListItemButton onClick={() => setOpen('polygon')}>
        <ListItemIcon>
          <Code />
        </ListItemIcon>
        <ListItemText primary="Geofence Input" />
      </ListItemButton>
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
      <ListSubheader disableGutters>Manage Current GeoJSON</ListSubheader>
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
      <ImportWizard />
    </List>
  )
}
