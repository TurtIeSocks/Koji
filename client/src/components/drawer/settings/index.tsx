import * as React from 'react'
import { List, ListItem, ListSubheader } from '@mui/material'

import { usePersist } from '@hooks/usePersist'

import Toggle from '../inputs/Toggle'
import BtnGroup from '../inputs/BtnGroup'
import DateTime from '../inputs/DateTime'

export default function Settings() {
  const pokestop = usePersist((s) => s.pokestop)
  const gym = usePersist((s) => s.gym)
  const spawnpoint = usePersist((s) => s.spawnpoint)
  const data = usePersist((s) => s.data)
  const nativeLeaflet = usePersist((s) => s.nativeLeaflet)
  const last_seen = usePersist((s) => s.last_seen)
  const setStore = usePersist((s) => s.setStore)

  return (
    <List>
      <ListSubheader disableGutters>Markers</ListSubheader>
      <Toggle field="pokestop" value={pokestop} setValue={setStore} />
      <Toggle field="gym" value={gym} setValue={setStore} />
      <Toggle field="spawnpoint" value={spawnpoint} setValue={setStore} />
      <DateTime field="last_seen" value={last_seen} setValue={setStore} />

      <Toggle field="nativeLeaflet" value={nativeLeaflet} setValue={setStore} />
      <ListItem>
        <BtnGroup
          field="data"
          value={data}
          setValue={setStore}
          buttons={['all', 'bound', 'area']}
        />
      </ListItem>
    </List>
  )
}
