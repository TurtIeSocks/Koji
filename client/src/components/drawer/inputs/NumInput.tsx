/* eslint-disable react/jsx-no-duplicate-props */
import * as React from 'react'
import { ListItem, ListItemText, TextField } from '@mui/material'

import { fromCamelCase, fromSnakeCase } from '@services/utils'
import { UsePersist, usePersist } from '@hooks/usePersist'
import { OnlyType } from '@assets/types'

export default function NumInput<
  T extends keyof OnlyType<UsePersist, number | ''>,
>({
  field,
  endAdornment,
  disabled = false,
}: {
  field: T
  disabled?: boolean
  endAdornment?: string
}) {
  const value = usePersist((s) => s[field])
  const setStore = usePersist((s) => s.setStore)

  return (
    <ListItem disabled={disabled}>
      <ListItemText
        primary={
          field.includes('_') ? fromSnakeCase(field) : fromCamelCase(field)
        }
      />
      <TextField
        name={field}
        value={value || ''}
        type="number"
        size="small"
        onChange={({ target }) => setStore(field, +target.value)}
        sx={{ width: '35%' }}
        inputProps={{ min: 0, max: 9999 }}
        InputProps={{ endAdornment }}
        disabled={disabled}
      />
    </ListItem>
  )
}
