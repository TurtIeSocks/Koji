import React from 'react'
import { List } from '@mui/material'

import ListSubheader from '../styled/Subheader'
import Toggle from './inputs/Toggle'
import { MultiOptionList } from './inputs/MultiOptions'
import NumInput from './inputs/NumInput'

export default function DrawingTab() {
  return (
    <List dense sx={{ width: 275 }}>
      <ListSubheader disableGutters>Drawing</ListSubheader>
      <Toggle field="snappable" />
      <Toggle field="continueDrawing" />
      <NumInput field="radius" />
      <MultiOptionList
        field="setActiveMode"
        label="Activate"
        buttons={['hover', 'click']}
        type="select"
        hideLabel
      />
    </List>
  )
}
