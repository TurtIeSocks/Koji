import * as React from 'react'
import {
  Divider,
  List,
  ListItemButton,
  ListItemIcon,
  ListItemText,
} from '@mui/material'
import Save from '@mui/icons-material/Save'
import AutoFix from '@mui/icons-material/AutoFixHigh'
import Code from '@mui/icons-material/Code'
import ChangeCircle from '@mui/icons-material/ChangeCircle'
import Fence from '@mui/icons-material/Fence'
import Route from '@mui/icons-material/Route'

import { useStatic } from '@hooks/useStatic'
import RawManager from '@components/dialogs/Manager'
import ImportWizard from '@components/dialogs/import/ImportWizard'
import ConvertDialog from '@components/dialogs/Convert'

import ExportRoute from '../../dialogs/ExportRoute'
import PolygonDialog from '../../dialogs/Polygon'
import InstanceSelect from './Instance'
import StyledSubheader from '../../styled/Subheader'

export default function ImportExport() {
  const [open, setOpen] = React.useState('')
  const [exportAll, setExportAll] = React.useState(false)
  const geojson = useStatic((s) => s.geojson)
  const setStatic = useStatic((s) => s.setStatic)

  return (
    <List dense sx={{ width: 275 }}>
      <StyledSubheader>Import</StyledSubheader>
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
        label="Import from Scanner"
      />
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
        label="Import from Kōji"
      />
      <Divider sx={{ my: 2 }} />
      <StyledSubheader>Export</StyledSubheader>
      <ListItemButton
        onClick={() => {
          setOpen('polygon')
          setExportAll(true)
        }}
      >
        <ListItemIcon>
          <Fence />
        </ListItemIcon>
        Export Polygons
      </ListItemButton>
      <ListItemButton onClick={() => setOpen('route')}>
        <ListItemIcon>
          <Route />
        </ListItemIcon>
        Export Routes
      </ListItemButton>
      <Divider sx={{ my: 2 }} />
      <StyledSubheader>Other</StyledSubheader>
      <ListItemButton onClick={() => setOpen('rawManager')}>
        <ListItemIcon>
          <Save />
        </ListItemIcon>
        <ListItemText primary="JSON Manager" />
      </ListItemButton>
      <ListItemButton
        onClick={() =>
          useStatic.setState((prev) => ({
            dialogs: { ...prev.dialogs, convert: true },
          }))
        }
      >
        <ListItemIcon>
          <ChangeCircle />
        </ListItemIcon>
        <ListItemText primary="Conversion Playground" />
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
      <ConvertDialog />
    </List>
  )
}
