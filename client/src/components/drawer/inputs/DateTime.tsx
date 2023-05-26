import * as React from 'react'
import dayjs from 'dayjs'
import TextField from '@mui/material/TextField'
import { DateTimePicker } from '@mui/x-date-pickers/DateTimePicker'

import { usePersist, type UsePersist } from '@hooks/usePersist'
import { fromCamelCase, fromSnakeCase } from '@services/utils'
import { ListItem } from '@mui/material'
import { OnlyType } from '@assets/types'

export default function DateTime<T extends keyof OnlyType<UsePersist, Date>>({
  field,
  disabled,
}: {
  field: T
  disabled?: boolean
}) {
  const value = usePersist((s) => s[field])

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
            usePersist.setState({
              [field]: newValue.set('second', 0).set('minute', 0).toDate(),
            })
          }
        }}
      />
    </ListItem>
  )
}
