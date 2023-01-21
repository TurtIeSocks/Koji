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
  const layerEditing = useStatic((s) => s.layerEditing)
  const updateButton = useStatic((s) => s.updateButton)

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
      </Collapse>
      <Collapse in={mode === 'route'}>
        <Divider sx={{ my: 2 }} />
        <ListSubheader>Routing</ListSubheader>
        <NumInput
          field="routing_time"
          endAdornment="s"
          disabled={mode !== 'route'}
        />
        <NumInput field="route_chunk_size" disabled={mode !== 'route'} />
      </Collapse>

      <Divider sx={{ my: 2 }} />
      <ListSubheader>Saving</ListSubheader>
      <Toggle field="save_to_db" label="Auto Save to Kōji Db" />

      {/* <NumInput field="generations" /> */}
      {/* <NumInput field="devices" disabled={mode !== 'route'} /> */}
      <ListItemButton
        color="primary"
        disabled={Object.values(layerEditing).some((v) => v) || !!updateButton}
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
