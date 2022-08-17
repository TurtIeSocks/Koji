import * as React from 'react'
import { ToggleButtonGroup, ToggleButton, ListItem } from '@mui/material'

import { type UseStore } from '@hooks/useStore'
import { fromCamelCase } from '@services/utils'

type Inputs = 'mode' | 'category' | 'data' | 'renderer'

interface Props<T extends Inputs> {
  field: T
  value: UseStore[T]
  setValue: (field: T, value: UseStore[T]) => void
  buttons: UseStore[T][]
  disabled?: boolean
}

export default function BtnGroup<T extends Inputs>({
  field,
  value,
  setValue,
  buttons,
  disabled = false,
}: Props<T>) {
  return (
    <ListItem disabled={disabled}>
      <ToggleButtonGroup
        size="small"
        color="primary"
        value={value}
        exclusive
        onChange={(_e, v: UseStore[T] | null) => {
          if (v) setValue(field, v)
        }}
        sx={{ mx: 'auto' }}
      >
        {buttons.map((m) => (
          <ToggleButton key={m} value={m}>
            {fromCamelCase(m)}
          </ToggleButton>
        ))}
      </ToggleButtonGroup>
    </ListItem>
  )
}
