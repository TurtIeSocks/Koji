/* eslint-disable react/no-array-index-key */
import React from 'react'
import { Divider, List } from '@mui/material'

import { usePersist } from '@hooks/usePersist'

import ListSubheader from '../styled/Subheader'
import Toggle from './inputs/Toggle'
import { MultiOptionList } from './inputs/MultiOptions'
import UserTextInput from './inputs/NumInput'
import { LineColorSelector } from './inputs/LineStringColor'

export default function DrawingTab() {
  const calculationMode = usePersist((s) => s.calculation_mode)
  return (
    <List dense>
      <ListSubheader disableGutters>Drawing</ListSubheader>
      <Toggle field="snappable" />
      <Toggle field="continueDrawing" />
      <Toggle field="keepCutoutsOnMerge" />
      <UserTextInput field="radius" disabled={calculationMode === 'S2'} />
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
