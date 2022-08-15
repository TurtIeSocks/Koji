import * as React from 'react'
import { capitalize, ListItem, ListItemText, TextField } from '@mui/material'

interface Props<T> {
  field: T
  value: number | ''
  setValue: (field: T, value: number | '') => void
}

export default function NumInput<T extends string>({
  field,
  value,
  setValue,
}: Props<T>) {
  return (
    <ListItem>
      <ListItemText
        primary={capitalize(field)}
        primaryTypographyProps={{ align: 'center' }}
      />
      <TextField
        name={field}
        value={value}
        type="number"
        size="small"
        onChange={({ target }) => setValue(field, +target.value || '')}
        sx={{ width: '50%' }}
      />
    </ListItem>
  )
}
