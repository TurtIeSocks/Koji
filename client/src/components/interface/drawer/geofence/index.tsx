import React from 'react'
import { Divider } from '@mui/material'

import { useStore } from '@hooks/useStore'

import InstanceSelect from './Instance'
import ListSubheader from '../../styled/Subheader'
import Toggle from '../inputs/Toggle'

export default function GeofenceTab() {
  const setStore = useStore((s) => s.setStore)
  const showCircles = useStore((s) => s.showCircles)
  const showLines = useStore((s) => s.showLines)
  const showPolygon = useStore((s) => s.showPolygon)
  const mode = useStore((s) => s.mode)

  return (
    <>
      <InstanceSelect />
      <Divider sx={{ my: 2 }} />
      <ListSubheader disableGutters>Vectors</ListSubheader>
      <Toggle field="showCircles" value={showCircles} setValue={setStore} />
      <Toggle
        field="showLines"
        value={showLines}
        setValue={setStore}
        disabled={mode === 'cluster'}
      />
      <Toggle field="showPolygon" value={showPolygon} setValue={setStore} />
    </>
  )
}
