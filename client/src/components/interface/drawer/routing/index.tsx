import React from 'react'
import { useStore } from '@hooks/useStore'
import { ListItem } from '@mui/material'

import ListSubheader from '../../styled/Subheader'
import NumInput from '../inputs/NumInput'
import BtnGroup from '../inputs/BtnGroup'

export default function EditTab() {
  const radius = useStore((s) => s.radius)
  const mode = useStore((s) => s.mode)
  const category = useStore((s) => s.category)
  const generations = useStore((s) => s.generations)
  const setStore = useStore((s) => s.setStore)
  const devices = useStore((s) => s.devices)

  return (
    <>
      <ListSubheader disableGutters>Routing</ListSubheader>
      <NumInput field="radius" value={radius} setValue={setStore} />
      <NumInput field="generations" value={generations} setValue={setStore} />
      <NumInput
        field="devices"
        value={devices}
        setValue={setStore}
        disabled={mode !== 'route'}
      />
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
    </>
  )
}
