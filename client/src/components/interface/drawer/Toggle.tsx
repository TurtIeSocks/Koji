import * as React from 'react'
import { ListItem, ListItemText, Switch } from '@mui/material'

import { type UseStore } from '@hooks/useStore'
import { fromCamelCase } from '@services/utils'

type Inputs =
  | 'pokestop'
  | 'spawnpoint'
  | 'gym'
  | 'showCircles'
  | 'showLines'
  | 'showPolygon'

interface Props<T extends Inputs> {
  field: T
  value: UseStore[T]
  setValue: (field: T, value: UseStore[T]) => void
  disabled?: boolean
}

export default function Toggle<T extends Inputs>({
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
      />
    </ListItem>
  )
}
