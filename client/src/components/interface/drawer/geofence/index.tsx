import React from 'react'
import { List, Divider } from '@mui/material'

import { useStore } from '@hooks/useStore'

import ListSubheader from '../../styled/Subheader'
import Toggle from '../inputs/Toggle'

export default function GeofenceTab() {
  const setStore = useStore((s) => s.setStore)
  const showCircles = useStore((s) => s.showCircles)
  const showLines = useStore((s) => s.showLines)
  const showPolygon = useStore((s) => s.showPolygon)
  const snappable = useStore((s) => s.snappable)
  const continueDrawing = useStore((s) => s.continueDrawing)

  return (
    <List dense>
      <Toggle field="snappable" value={snappable} setValue={setStore} />
      <Toggle
        field="continueDrawing"
        value={continueDrawing}
        setValue={setStore}
      />
      <Divider sx={{ my: 2 }} />
      <ListSubheader disableGutters>Vectors</ListSubheader>
      <Toggle field="showCircles" value={showCircles} setValue={setStore} />
      <Toggle field="showLines" value={showLines} setValue={setStore} />
      <Toggle field="showPolygon" value={showPolygon} setValue={setStore} />
    </List>
  )
}
