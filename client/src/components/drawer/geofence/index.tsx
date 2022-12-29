import React from 'react'
import { List, Divider } from '@mui/material'

import { usePersist } from '@hooks/usePersist'

import ListSubheader from '../../styled/Subheader'
import Toggle from '../inputs/Toggle'

export default function GeofenceTab() {
  const setStore = usePersist((s) => s.setStore)
  const showCircles = usePersist((s) => s.showCircles)
  const showLines = usePersist((s) => s.showLines)
  const showPolygon = usePersist((s) => s.showPolygons)
  const snappable = usePersist((s) => s.snappable)
  const continueDrawing = usePersist((s) => s.continueDrawing)

  return (
    <List dense>
      <ListSubheader disableGutters>Drawing Options</ListSubheader>
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
      <Toggle field="showPolygons" value={showPolygon} setValue={setStore} />
    </List>
  )
}
