import * as React from 'react'
import { ListItem, ListItemText, Switch } from '@mui/material'

import { type UsePersist } from '@hooks/usePersist'
import { fromCamelCase, fromSnakeCase } from '@services/utils'

interface Props<T extends keyof UsePersist> {
  field: T
  value: boolean
  setValue: (field: Props<T>['field'], value: Props<T>['value']) => void
  disabled?: boolean
}

export default function Toggle<T extends keyof UsePersist>({
  field,
  value,
  setValue,
  disabled = false,
}: Props<T>) {
  return (
    <ListItem disabled={disabled}>
      <ListItemText
        primary={
          field.includes('_') ? fromSnakeCase(field) : fromCamelCase(field)
        }
      />
      <Switch
        edge="end"
        onChange={(_e, v) => setValue(field, v)}
        checked={value}
        disabled={disabled}
      />
    </ListItem>
  )
}
