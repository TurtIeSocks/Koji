import * as React from 'react'
import dayjs from 'dayjs'
import TextField from '@mui/material/TextField'
import { DateTimePicker } from '@mui/x-date-pickers/DateTimePicker'

import { type UseStore } from '@hooks/useStore'
import { fromCamelCase, fromSnakeCase } from '@services/utils'
import { ListItem } from '@mui/material'

interface Props<T extends keyof UseStore> {
  field: T
  value: Date
  setValue: (field: Props<T>['field'], value: Props<T>['value']) => void
  disabled?: boolean
}

export default function DateTime<T extends keyof UseStore>({
  field,
  value,
  setValue,
  disabled,
}: Props<T>) {
  return (
    <ListItem>
      <DateTimePicker
        disabled={disabled}
        disableFuture
        views={['year', 'month', 'day', 'hours']}
        label={
          field.includes('_') ? fromSnakeCase(field) : fromCamelCase(field)
        }
        renderInput={(params) => (
          <TextField fullWidth sx={{ my: 1 }} size="small" {...params} />
        )}
        value={dayjs(value)}
        onChange={(newValue) => {
          if (newValue) {
            setValue(field, newValue.set('second', 0).set('minute', 0).toDate())
          }
        }}
      />
    </ListItem>
  )
}
