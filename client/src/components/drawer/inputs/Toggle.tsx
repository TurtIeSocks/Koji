import * as React from 'react'
import { ListItem, ListItemText, Switch } from '@mui/material'

import { usePersist, type UsePersist } from '@hooks/usePersist'
import { fromCamelCase, fromSnakeCase } from '@services/utils'
import { OnlyType } from '@assets/types'

export default function Toggle<T extends keyof OnlyType<UsePersist, boolean>>({
  field,
  label,
  disabled = false,
}: {
  field: T
  disabled?: boolean
  label?: string
}) {
  const value = usePersist((s) => s[field])

  return (
    <ListItem disabled={disabled}>
      <ListItemText
        primary={
          label ??
          (field.includes('_') ? fromSnakeCase(field) : fromCamelCase(field))
        }
      />
      <Switch
        edge="end"
        onChange={(_e, v) => usePersist.setState({ [field]: v })}
        checked={!!value}
        disabled={disabled}
      />
    </ListItem>
  )
}
