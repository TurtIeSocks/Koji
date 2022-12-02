/* eslint-disable react/jsx-no-duplicate-props */
import * as React from 'react'
import { ListItem, ListItemText, TextField } from '@mui/material'

import { fromCamelCase } from '@services/utils'

interface Props<T> {
  field: T
  value: number | ''
  setValue: (field: T, value: number | '') => void
  disabled?: boolean
  endAdornment?: string
}

export default function NumInput<T extends string>({
  field,
  value,
  setValue,
  endAdornment,
  disabled = false,
}: Props<T>) {
  return (
    <ListItem disabled={disabled}>
      <ListItemText primary={fromCamelCase(field).replace(/_/g, ' ')} />
      <TextField
        name={field}
        value={value}
        type="number"
        size="small"
        onChange={({ target }) =>
          setValue(field, +target.value === 0 ? 0 : +target.value || '')
        }
        sx={{ width: '35%' }}
        inputProps={{ min: 0, max: 9999 }}
        InputProps={{ endAdornment }}
        disabled={disabled}
      />
    </ListItem>
  )
}
