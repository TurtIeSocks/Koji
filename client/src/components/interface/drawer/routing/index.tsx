import React from 'react'
import { ListItem, ListItemButton, ListItemText } from '@mui/material'

import { useStatic } from '@hooks/useStatic'
import { useStore } from '@hooks/useStore'

import ListSubheader from '../../styled/Subheader'
import NumInput from '../inputs/NumInput'
import BtnGroup from '../inputs/BtnGroup'
import Toggle from '../inputs/Toggle'

export default function EditTab() {
  const radius = useStore((s) => s.radius)
  const mode = useStore((s) => s.mode)
  const category = useStore((s) => s.category)
  const generations = useStore((s) => s.generations)
  const setStore = useStore((s) => s.setStore)
  const devices = useStore((s) => s.devices)
  const min_points = useStore((s) => s.min_points)
  const fast = useStore((s) => s.fast)
  const autoMode = useStore((s) => s.autoMode)

  const setStatic = useStatic((s) => s.setStatic)

  return (
    <>
      <ListSubheader disableGutters>Routing</ListSubheader>
      <Toggle field="autoMode" value={autoMode} setValue={setStore} />
      <NumInput field="radius" value={radius} setValue={setStore} />
      <NumInput field="generations" value={generations} setValue={setStore} />
      <NumInput
        field="devices"
        value={devices}
        setValue={setStore}
        disabled={mode !== 'route'}
      />
      <NumInput field="min_points" value={min_points} setValue={setStore} />
      <Toggle field="fast" value={fast} setValue={setStore} />
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
      {!autoMode && (
        <ListItemButton
          color="primary"
          onClick={() => setStatic('forceFetch', (prev) => !prev)}
        >
          <ListItemText primary="Update" color="blue" />
        </ListItemButton>
      )}
    </>
  )
}
