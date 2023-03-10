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
import { useShapes } from '@hooks/useShapes'
import { KojiKey } from '@assets/types'
import RawManager from '@components/dialogs/Manager'
import ImportWizard from '@components/dialogs/import/ImportWizard'
import ConvertDialog from '@components/dialogs/Convert'
import { useImportExport } from '@hooks/useImportExport'

import InstanceSelect from '../inputs/Instance'
import StyledSubheader from '../../styled/Subheader'
import SelectProject from '../inputs/SelectProject'

export default function ImportExport() {
  const geojson = useStatic((s) => s.geojson)
  const setStatic = useStatic((s) => s.setStatic)

  const points = geojson.features.filter(
    (feat) => feat.geometry.type === 'Point',
  )
  const addPoint =
    points.length &&
    points.every(
      (feat, _, ref) =>
        feat.properties.__multipoint_id === ref[0].properties.__multipoint_id,
    )
      ? (points[0].properties.__multipoint_id as KojiKey)
      : ''

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
      <ListItemButton
        onClick={() => {
          useImportExport.setState({ open: 'importPolygon' })
        }}
      >
        <ListItemIcon>
          <Code />
        </ListItemIcon>
        <ListItemText primary="Import Polygons" />
      </ListItemButton>

      <ListItemButton
        onClick={() => {
          useImportExport.setState({ open: 'importRoute' })
        }}
      >
        <ListItemIcon>
          <Code />
        </ListItemIcon>
        <ListItemText primary="Import Routes" />
      </ListItemButton>

      <InstanceSelect
        controlled
        initialState={[
          ...(addPoint && addPoint.endsWith('__SCANNER') ? [addPoint] : []),
          ...geojson.features
            .filter(
              (feat) =>
                feat.geometry.type !== 'LineString' &&
                feat.geometry.type !== 'Point' &&
                typeof feat.id === 'string' &&
                feat.id.endsWith('__SCANNER'),
            )
            .map((feat) => feat.id as KojiKey),
        ]}
        label="Import from Scanner"
      />
      <InstanceSelect
        koji
        controlled
        initialState={[
          ...(addPoint && addPoint.endsWith('__KOJI') ? [addPoint] : []),
          ...geojson.features
            .filter(
              (feat) =>
                feat.geometry.type !== 'LineString' &&
                feat.geometry.type !== 'Point' &&
                typeof feat.id === 'string' &&
                feat.id.endsWith('__KOJI'),
            )
            .map((feat) => feat.id as KojiKey),
        ]}
        label="Import from Kōji"
      />
      <SelectProject />
      <Divider sx={{ my: 2 }} />
      <StyledSubheader>Export</StyledSubheader>
      <ListItemButton
        onClick={() => {
          useImportExport.setState({
            open: 'exportPolygon',
            feature: {
              ...geojson,
              features: geojson.features.filter((feat) =>
                feat.geometry.type.includes('Polygon'),
              ),
            },
          })
        }}
      >
        <ListItemIcon>
          <Fence />
        </ListItemIcon>
        Export Polygons
      </ListItemButton>
      <ListItemButton
        onClick={() => {
          useImportExport.setState({
            open: 'exportRoute',
            feature: {
              ...geojson,
              features: geojson.features.filter(
                (feat) => feat.geometry.type === 'MultiPoint',
              ),
            },
          })
        }}
      >
        <ListItemIcon>
          <Route />
        </ListItemIcon>
        Export Routes
      </ListItemButton>
      <Divider sx={{ my: 2 }} />
      <StyledSubheader>Other</StyledSubheader>
      <ListItemButton
        onClick={() => {
          useShapes.getState().setters.activeRoute()
          useStatic.setState((prev) => ({
            dialogs: { ...prev.dialogs, manager: true },
          }))
        }}
      >
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
      <RawManager />
      <ImportWizard />
      <ConvertDialog />
    </List>
  )
}
