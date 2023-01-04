import React from 'react'
import { List, Divider, ListItem } from '@mui/material'

import ListSubheader from '../../styled/Subheader'
import Toggle from '../inputs/Toggle'
import MultiOptions from '../inputs/MultiOptions'

export default function DrawingTab() {
  return (
    <List dense>
      <ListSubheader disableGutters>Drawing Options</ListSubheader>
      <Toggle field="snappable" />
      <Toggle field="continueDrawing" />
      <ListItem>
        Activate:
        <MultiOptions field="setActiveMode" buttons={['hover', 'click']} />
      </ListItem>
      <Divider sx={{ my: 2 }} />
      <ListSubheader disableGutters>Vectors</ListSubheader>
      <Toggle field="showCircles" />
      <Toggle field="showLines" />
      <Toggle field="showPolygons" />
      <Toggle field="showArrows" />
    </List>
  )
}
