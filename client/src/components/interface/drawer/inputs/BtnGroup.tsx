import * as React from 'react'
import { ToggleButtonGroup, ToggleButton } from '@mui/material'

import { type UseStore } from '@hooks/useStore'
import { fromCamelCase } from '@services/utils'

interface Props<T extends keyof UseStore, K extends string> {
  field: T
  value: K
  setValue: (field: Props<T, K>['field'], value: Props<T, K>['value']) => void
  buttons: K[]
  disabled?: boolean
}

export default function BtnGroup<T extends keyof UseStore, K extends string>({
  field,
  value,
  setValue,
  buttons,
  disabled = false,
}: Props<T, K>) {
  return (
    <ToggleButtonGroup
      size="small"
      color="primary"
      value={value}
      exclusive
      onChange={(_e, v) => setValue(field, v)}
      sx={{ mx: 'auto' }}
      disabled={disabled}
    >
      {buttons.map((m) => (
        <ToggleButton key={m} value={m} disabled={disabled}>
          {fromCamelCase(m)}
        </ToggleButton>
      ))}
    </ToggleButtonGroup>
  )
}
