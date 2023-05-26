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

type FieldType = keyof OnlyType<UsePersist, string | number>

interface Props<T extends FieldType, K extends UsePersist[T]> {
  field: T
  buttons: readonly K[]
  disabled?: boolean
  type?: 'button' | 'select'
  label?: string
  hideLabel?: boolean
  itemLabel?: (item: K) => string
}

export default function MultiOptions<
  T extends FieldType,
  K extends UsePersist[T],
>({
  field,
  buttons,
  disabled = false,
  type = 'button',
  label = '',
  hideLabel = !label,
  itemLabel = (item: number | string) =>
    typeof item === 'string'
      ? item.includes('_')
        ? fromSnakeCase(item)
        : fromCamelCase(item)
      : `${item}`,
}: Props<T, K>) {
  const [value, setValue] = usePersist((s) => [s[field], usePersist.setState])

  return type === 'button' ? (
    <ToggleButtonGroup
      size="small"
      color="primary"
      value={value}
      exclusive
      onChange={(_e, v) => setValue({ [field]: v })}
      sx={{ mx: 'auto' }}
      disabled={disabled}
    >
      {buttons.map((m) => (
        <ToggleButton key={m} value={m} disabled={disabled}>
          {itemLabel(m)}
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
        onChange={({ target }) => setValue({ [field]: target.value as K })} // Mui y u like this
        sx={{ mx: 'auto', minWidth: 150 }}
        disabled={disabled}
      >
        {buttons.map((m) => (
          <MenuItem key={m} value={m}>
            {itemLabel(m)}
          </MenuItem>
        ))}
      </Select>
    </FormControl>
  )
}

export function MultiOptionList<T extends FieldType, K extends UsePersist[T]>({
  field,
  label,
  ...rest
}: Props<T, K>) {
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
