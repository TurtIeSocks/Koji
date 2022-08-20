import * as React from 'react'
import { ListItem, ListItemText, Switch } from '@mui/material'

import { type UseStore } from '@hooks/useStore'
import { fromCamelCase } from '@services/utils'

interface Props<T extends keyof UseStore> {
  field: T
  value: boolean
  setValue: (field: Props<T>['field'], value: Props<T>['value']) => void
  disabled?: boolean
}

export default function Toggle<T extends keyof UseStore>({
  field,
  value,
  setValue,
  disabled = false,
}: Props<T>) {
  return (
    <ListItem disabled={disabled}>
      <ListItemText primary={fromCamelCase(field)} />
      <Switch
        edge="end"
        onChange={(_e, v) => setValue(field, v)}
        checked={value}
        disabled={disabled}
      />
    </ListItem>
  )
}
