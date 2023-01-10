import React from 'react'
import { List, ListItem, ListItemButton, ListItemText } from '@mui/material'

import { useStatic } from '@hooks/useStatic'
import { usePersist } from '@hooks/usePersist'
import { clusteringRouting } from '@services/fetches'

import ListSubheader from '../../styled/Subheader'
import NumInput from '../inputs/NumInput'
import MultiOptions from '../inputs/MultiOptions'
import Toggle from '../inputs/Toggle'

export default function EditTab() {
  const mode = usePersist((s) => s.mode)

  const layerEditing = useStatic((s) => s.layerEditing)

  return (
    <List dense>
      <ListSubheader disableGutters>Routing</ListSubheader>
      <NumInput field="radius" />
      <NumInput field="min_points" />
      {/* <NumInput field="generations" /> */}
      <NumInput
        field="routing_time"
        endAdornment="s"
        disabled={mode !== 'route'}
      />
      <NumInput field="route_chunk_size" disabled={mode !== 'route'} />
      {/* <NumInput field="devices" disabled={mode !== 'route'} /> */}
      <Toggle field="fast" />
      <Toggle field="only_unique" />
      <ListItem disabled={mode === 'bootstrap'}>
        <MultiOptions
          field="category"
          buttons={['pokestop', 'gym', 'spawnpoint']}
          disabled={mode === 'bootstrap'}
        />
      </ListItem>
      <ListItem>
        <MultiOptions
          field="mode"
          buttons={['cluster', 'route', 'bootstrap']}
        />
      </ListItem>
      <Toggle field="save_to_db" disabled />
      <ListItemButton
        color="primary"
        disabled={Object.values(layerEditing).some((v) => v)}
        onClick={clusteringRouting}
      >
        <ListItemText primary="Update" color="blue" />
      </ListItemButton>
    </List>
  )
}
