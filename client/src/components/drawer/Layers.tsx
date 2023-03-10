import * as React from 'react'
import { Divider, List } from '@mui/material'

import Toggle from './inputs/Toggle'
import ListSubheader from '../styled/Subheader'

export default function Layers() {
  return (
    <List dense>
      <ListSubheader disableGutters>Vectors</ListSubheader>
      <Toggle field="showCircles" />
      <Toggle field="showLines" />
      <Toggle field="showPolygons" />
      <Toggle field="showArrows" />
      <Divider sx={{ my: 2 }} />
      <ListSubheader disableGutters>Markers</ListSubheader>
      <Toggle field="gym" />
      <Toggle field="spawnpoint" />
      <Toggle field="pokestop" />
      <Toggle field="pokestopRange" />
    </List>
  )
}
