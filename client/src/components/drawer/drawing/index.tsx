import React from 'react'
import {
  List,
  Divider,
  ListItem,
  ListItemButton,
  ListItemIcon,
  ListItemText,
} from '@mui/material'
import Check from '@mui/icons-material/Check'
import Clear from '@mui/icons-material/Clear'

import { useStatic } from '@hooks/useStatic'

import ListSubheader from '../../styled/Subheader'
import Toggle from '../inputs/Toggle'
import MultiOptions from '../inputs/MultiOptions'

export default function DrawingTab() {
  const combinePolyMode = useStatic((s) => s.combinePolyMode)
  const setStatic = useStatic((s) => s.setStatic)

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
      <Divider sx={{ my: 2 }} />
      <ListSubheader disableGutters>Utilities</ListSubheader>
      <ListItemButton
        color={combinePolyMode ? 'secondary' : 'primary'}
        onClick={() => setStatic('combinePolyMode', (prev) => !prev)}
      >
        <ListItemText>Combine Polygon Mode</ListItemText>
        <ListItemIcon>{combinePolyMode ? <Check /> : <Clear />}</ListItemIcon>
      </ListItemButton>
    </List>
  )
}
