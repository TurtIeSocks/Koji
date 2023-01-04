import * as React from 'react'
import {
  ToggleButtonGroup,
  ToggleButton,
  Select,
  MenuItem,
} from '@mui/material'

import { usePersist, type UsePersist } from '@hooks/usePersist'
import { fromCamelCase, fromSnakeCase } from '@services/utils'
import { OnlyType } from '@assets/types'

export default function MultiOptions<
  T extends keyof OnlyType<UsePersist, string>,
  K extends UsePersist[T],
>({
  field,
  buttons,
  disabled = false,
  type = 'button',
}: {
  field: T
  buttons: K[]
  disabled?: boolean
  type?: 'button' | 'select'
}) {
  const value = usePersist((s) => s[field])
  const setStore = usePersist((s) => s.setStore)

  return type === 'button' ? (
    <ToggleButtonGroup
      size="small"
      color="primary"
      value={value}
      exclusive
      onChange={(_e, v) => setStore(field, v)}
      sx={{ mx: 'auto' }}
      disabled={disabled}
    >
      {buttons.map((m) => (
        <ToggleButton key={m} value={m} disabled={disabled}>
          {m.includes('_') ? fromSnakeCase(m) : fromCamelCase(m)}
        </ToggleButton>
      ))}
    </ToggleButtonGroup>
  ) : (
    <Select
      size="small"
      color="primary"
      value={value}
      onChange={({ target }) => setStore(field, target.value as K)} // Mui y u like this
      sx={{ mx: 'auto' }}
      disabled={disabled}
    >
      {buttons.map((m) => (
        <MenuItem key={m} value={m}>
          {m.includes('_') ? fromSnakeCase(m) : fromCamelCase(m)}
        </MenuItem>
      ))}
    </Select>
  )
}
