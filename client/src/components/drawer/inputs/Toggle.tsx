import * as React from 'react'
import { ListItem, ListItemText, Switch } from '@mui/material'

import { usePersist, type UsePersist } from '@hooks/usePersist'
import { fromCamelCase, fromSnakeCase } from '@services/utils'
import { OnlyType } from '@assets/types'

export default function Toggle<T extends keyof OnlyType<UsePersist, boolean>>({
  field,
  disabled = false,
}: {
  field: T
  disabled?: boolean
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
      <Switch
        edge="end"
        onChange={(_e, v) => setStore(field, v)}
        checked={!!value}
        disabled={disabled}
      />
    </ListItem>
  )
}
