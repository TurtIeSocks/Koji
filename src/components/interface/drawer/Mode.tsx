import * as React from 'react'
import { ToggleButtonGroup, ToggleButton, ListItem } from '@mui/material'

import { type UseStore } from '@hooks/useStore'

interface Props {
  value: UseStore['apiSettings']['mode']
  setValue: (
    field: keyof UseStore['apiSettings'],
    value: UseStore['apiSettings']['mode'],
  ) => void
}

export default function Mode({ value, setValue }: Props) {
  return (
    <ListItem>
      <ToggleButtonGroup
        size="small"
        color="primary"
        value={value}
        exclusive
        onChange={(_e, v: UseStore['apiSettings']['mode'] | null) => {
          if (v) setValue('mode', v)
        }}
        sx={{ mx: 'auto' }}
      >
        {['cluster', 'route', 'bootstrap'].map((m) => (
          <ToggleButton key={m} value={m}>
            {m}
          </ToggleButton>
        ))}
      </ToggleButtonGroup>
    </ListItem>
  )
}
