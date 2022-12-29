import React from 'react'
import { List, ListItem, ListItemButton, ListItemText } from '@mui/material'

import { useStatic } from '@hooks/useStatic'
import { usePersist } from '@hooks/usePersist'

import ListSubheader from '../../styled/Subheader'
import NumInput from '../inputs/NumInput'
import BtnGroup from '../inputs/BtnGroup'
import Toggle from '../inputs/Toggle'

export default function EditTab() {
  const radius = usePersist((s) => s.radius)
  const mode = usePersist((s) => s.mode)
  const category = usePersist((s) => s.category)
  const only_unique = usePersist((s) => s.only_unique)
  // const generations = useStore((s) => s.generations)
  const setStore = usePersist((s) => s.setStore)
  // const devices = useStore((s) => s.devices)
  const min_points = usePersist((s) => s.min_points)
  const fast = usePersist((s) => s.fast)
  const autoMode = usePersist((s) => s.autoMode)
  const routing_time = usePersist((s) => s.routing_time)
  const save_to_db = usePersist((s) => s.save_to_db)
  const route_chunk_size = usePersist((s) => s.route_chunk_size)

  const layerEditing = useStatic((s) => s.layerEditing)
  const setStatic = useStatic((s) => s.setStatic)

  return (
    <List dense>
      <ListSubheader disableGutters>Routing</ListSubheader>
      <Toggle field="autoMode" value={autoMode} setValue={setStore} />
      <NumInput field="radius" value={radius} setValue={setStore} />
      <NumInput field="min_points" value={min_points} setValue={setStore} />
      {/* <NumInput field="generations" value={generations} setValue={setStore} /> */}
      <NumInput
        field="routing_time"
        value={routing_time}
        setValue={setStore}
        endAdornment="s"
        disabled={mode !== 'route'}
      />
      <NumInput
        field="route_chunk_size"
        value={route_chunk_size}
        setValue={setStore}
        disabled={mode !== 'route'}
      />
      {/* <NumInput
        field="devices"
        value={devices}
        setValue={setStore}
        disabled={mode !== 'route'}
      /> */}
      <Toggle field="fast" value={fast} setValue={setStore} />
      <Toggle field="only_unique" value={only_unique} setValue={setStore} />
      <ListItem disabled={mode === 'bootstrap'}>
        <BtnGroup
          field="category"
          value={category}
          setValue={setStore}
          buttons={['pokestop', 'gym', 'spawnpoint']}
          disabled={mode === 'bootstrap'}
        />
      </ListItem>
      <ListItem>
        <BtnGroup
          field="mode"
          value={mode}
          setValue={setStore}
          buttons={['cluster', 'route', 'bootstrap']}
        />
      </ListItem>
      <Toggle
        field="save_to_db"
        value={save_to_db}
        setValue={setStore}
        disabled
      />
      {!autoMode && (
        <ListItemButton
          color="primary"
          disabled={Object.values(layerEditing).some((v) => v)}
          onClick={() => {
            setStatic('forceFetch', (prev) => !prev)
          }}
        >
          <ListItemText primary="Update" color="blue" />
        </ListItemButton>
      )}
    </List>
  )
}
