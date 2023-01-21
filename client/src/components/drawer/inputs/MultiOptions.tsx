import * as React from 'react'
import {
  ToggleButtonGroup,
  ToggleButton,
  Select,
  MenuItem,
  FormControl,
  InputLabel,
  ListItem,
  ListItemText,
} from '@mui/material'

import { usePersist, type UsePersist } from '@hooks/usePersist'
import { fromCamelCase, fromSnakeCase } from '@services/utils'
import { OnlyType } from '@assets/types'

interface Props<
  T extends keyof OnlyType<UsePersist, string>,
  K extends UsePersist[T],
> {
  field: T
  buttons: K[]
  disabled?: boolean
  type?: 'button' | 'select'
  label?: string
  hideLabel?: boolean
}

export default function MultiOptions<
  T extends keyof OnlyType<UsePersist, string>,
  K extends UsePersist[T],
>({
  field,
  buttons,
  disabled = false,
  type = 'button',
  label = '',
  hideLabel = !label,
}: Props<T, K>) {
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
    <FormControl>
      {!hideLabel && (
        <InputLabel id="multi-option-select-label">{label}</InputLabel>
      )}
      <Select
        labelId="multi-option-select-label"
        size="small"
        label={hideLabel ? undefined : label}
        value={value}
        color="primary"
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
    </FormControl>
  )
}

export function MultiOptionList<
  T extends keyof OnlyType<UsePersist, string>,
  K extends UsePersist[T],
>({ field, label, ...rest }: Props<T, K>) {
  const display = label || field
  return (
    <ListItem>
      <ListItemText
        primary={
          display.includes('_')
            ? fromSnakeCase(display)
            : fromCamelCase(display)
        }
      />
      <MultiOptions field={field} label={label} {...rest} />
    </ListItem>
  )
}
