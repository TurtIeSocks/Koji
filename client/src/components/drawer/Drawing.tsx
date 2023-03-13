/* eslint-disable react/no-array-index-key */
import React from 'react'
import { Divider, List } from '@mui/material'
import ListSubheader from '../styled/Subheader'
import Toggle from './inputs/Toggle'
import { MultiOptionList } from './inputs/MultiOptions'
import NumInput from './inputs/NumInput'
import { LineColorSelector } from './inputs/LineStringColor'

export default function DrawingTab() {
  return (
    <List dense>
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
      <Divider sx={{ my: 2 }} />
      <LineColorSelector />
    </List>
  )
}
