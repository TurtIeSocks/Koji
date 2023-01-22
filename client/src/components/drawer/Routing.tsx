import React from 'react'
import {
  Collapse,
  Divider,
  List,
  ListItemButton,
  ListItemIcon,
  ListItemText,
} from '@mui/material'
import Update from '@mui/icons-material/Update'

import { useStatic } from '@hooks/useStatic'
import { usePersist } from '@hooks/usePersist'
import { clusteringRouting } from '@services/fetches'

import ListSubheader from '../styled/Subheader'
import NumInput from './inputs/NumInput'
import { MultiOptionList } from './inputs/MultiOptions'
import Toggle from './inputs/Toggle'

export default function RoutingTab() {
  const mode = usePersist((s) => s.mode)
  const [layerEditing, updateButton] = useStatic((s) => [
    s.layerEditing,
    s.updateButton,
  ])

  return (
    <List dense sx={{ width: 275 }}>
      <ListSubheader>Calculation Modes</ListSubheader>
      <MultiOptionList
        field="category"
        buttons={['pokestop', 'gym', 'spawnpoint']}
        disabled={mode === 'bootstrap'}
        type="select"
      />
      <MultiOptionList
        field="mode"
        buttons={['cluster', 'route', 'bootstrap']}
        type="select"
      />
      <Divider sx={{ my: 2 }} />

      <ListSubheader>Clustering</ListSubheader>
      <NumInput field="radius" />
      <Collapse in={mode !== 'bootstrap'}>
        <NumInput field="min_points" />
        <Toggle field="fast" />
        <Toggle field="only_unique" />
        {/* <NumInput field="generations" /> */}
        {/* <NumInput field="devices" disabled={mode !== 'route'} /> */}
      </Collapse>
      <Divider sx={{ my: 2 }} />

      <Collapse in={mode === 'route'}>
        <ListSubheader>Routing</ListSubheader>
        <NumInput
          field="routing_time"
          endAdornment="s"
          disabled={mode !== 'route'}
        />
        <NumInput field="route_chunk_size" disabled={mode !== 'route'} />
        <Divider sx={{ my: 2 }} />
      </Collapse>

      <ListSubheader>Saving</ListSubheader>
      <Toggle field="save_to_db" label="Save to Kōji Db" />
      <Toggle
        field="save_to_scanner"
        label="Save to Scanner Db"
        disabled={!useStatic.getState().dangerous}
      />
      <Toggle field="skipRendering" />
      <ListItemButton
        color="primary"
        disabled={
          Object.values(layerEditing).some((v) => v) ||
          !!updateButton ||
          !useStatic.getState().geojson.features.length
        }
        onClick={async () => {
          useStatic.setState({ updateButton: true })
          await clusteringRouting().then(() => {
            useStatic.setState({ updateButton: false })
          })
        }}
      >
        <ListItemIcon>
          <Update color="secondary" />
        </ListItemIcon>
        <ListItemText
          primary="Update"
          primaryTypographyProps={{ color: 'secondary' }}
        />
      </ListItemButton>
    </List>
  )
}
