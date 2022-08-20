import React from 'react'
import { List, Divider, ListSubheader, type SxProps } from '@mui/material'

import { useStore } from '@hooks/useStore'

import NumInput from '../inputs/NumInput'
import BtnGroup from '../inputs/BtnGroup'
import Toggle from '../inputs/Toggle'

const subSx: SxProps = {
  textAlign: 'center',
  lineHeight: 2,
  fontWeight: 'bold',
}

export default function EditTab() {
  const radius = useStore((s) => s.radius)
  const mode = useStore((s) => s.mode)
  const category = useStore((s) => s.category)
  const generations = useStore((s) => s.generations)
  const setStore = useStore((s) => s.setStore)
  const pokestop = useStore((s) => s.pokestop)
  const gym = useStore((s) => s.gym)
  const spawnpoint = useStore((s) => s.spawnpoint)
  const data = useStore((s) => s.data)
  const showCircles = useStore((s) => s.showCircles)
  const showLines = useStore((s) => s.showLines)
  const showPolygon = useStore((s) => s.showPolygon)
  const nativeLeaflet = useStore((s) => s.nativeLeaflet)
  const devices = useStore((s) => s.devices)

  return (
    <List dense>
      <ListSubheader disableGutters sx={subSx}>
        Routing
      </ListSubheader>
      <NumInput field="radius" value={radius} setValue={setStore} />
      <NumInput field="generations" value={generations} setValue={setStore} />
      <NumInput
        field="devices"
        value={devices}
        setValue={setStore}
        disabled={mode !== 'route'}
      />
      <BtnGroup
        field="category"
        value={category}
        setValue={setStore}
        buttons={['pokestop', 'gym', 'spawnpoint']}
        disabled={mode === 'bootstrap'}
      />
      <BtnGroup
        field="mode"
        value={mode}
        setValue={setStore}
        buttons={['cluster', 'route', 'bootstrap']}
      />
      <Divider sx={{ my: 2 }} />
      <ListSubheader disableGutters sx={subSx}>
        Markers
      </ListSubheader>
      <Toggle field="pokestop" value={pokestop} setValue={setStore} />
      <Toggle field="gym" value={gym} setValue={setStore} />
      <Toggle field="spawnpoint" value={spawnpoint} setValue={setStore} />
      <Toggle field="nativeLeaflet" value={nativeLeaflet} setValue={setStore} />
      <BtnGroup
        field="data"
        value={data}
        setValue={setStore}
        buttons={['all', 'bound', 'area']}
      />
      <Divider sx={{ my: 2 }} />
      <ListSubheader disableGutters sx={subSx}>
        Vectors
      </ListSubheader>
      <Toggle field="showCircles" value={showCircles} setValue={setStore} />
      <Toggle
        field="showLines"
        value={showLines}
        setValue={setStore}
        disabled={mode === 'cluster'}
      />
      <Toggle field="showPolygon" value={showPolygon} setValue={setStore} />
    </List>
  )
}
