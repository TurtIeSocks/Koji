/* eslint-disable react/jsx-no-duplicate-props */
import * as React from 'react'
import { ListItem, ListItemText, TextField } from '@mui/material'

import { fromCamelCase, fromSnakeCase } from '@services/utils'
import { UsePersist, usePersist } from '@hooks/usePersist'
import { OnlyType } from '@assets/types'

export default function UserTextInput<
  U extends UsePersist[T],
  T extends keyof Omit<OnlyType<UsePersist, U | ''>, 's2_level' | 's2_size'>,
>({
  field,
  label,
  helperText,
  endAdornment,
  disabled = false,
  min,
  max,
}: {
  field: T
  label?: string
  helperText?: string
  disabled?: boolean
  endAdornment?: string
  min?: U extends number ? number : never
  max?: U extends number ? number : never
}) {
  const value = usePersist((s) => s[field])
  const isNumber = typeof value === 'number'
  const finalLabel =
    label ?? (field.includes('_') ? fromSnakeCase(field) : fromCamelCase(field))

  return (
    <ListItem disabled={disabled}>
      {isNumber && <ListItemText primary={finalLabel} />}
      <TextField
        name={field}
        value={value || ''}
        fullWidth
        type={isNumber ? 'number' : 'text'}
        size="small"
        onChange={({ target }) =>
          usePersist.setState({
            [field]: isNumber ? +target.value : target.value,
          })
        }
        label={isNumber ? undefined : finalLabel}
        sx={{ width: isNumber ? '35%' : '100%' }}
        inputProps={{ min: min || 0, max: max || 9999 }}
        InputProps={{ endAdornment }}
        disabled={disabled}
        helperText={helperText}
      />
    </ListItem>
  )
}
