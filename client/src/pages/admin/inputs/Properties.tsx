import * as React from 'react'
import { useInput } from 'react-admin'

import { ColorInput } from 'react-admin-color-picker'
import { Switch, TextField } from '@mui/material'

export function BoolInputExpanded({
  source,
  defaultValue,
  name,
  ...props
}: {
  source: string
  name?: string
  defaultValue: boolean
  label?: string
}) {
  const { id, field } = useInput({ source })

  React.useEffect(() => {
    if (typeof field.value !== 'boolean') {
      field.onChange(defaultValue)
    }
  }, [name, defaultValue])

  return (
    <Switch
      id={id}
      {...props}
      {...field}
      onChange={({ target }) => {
        field.onChange({
          target: {
            value: target.checked,
          },
        })
      }}
      checked={!!(field.value ?? defaultValue)}
    />
  )
}

export function TextInputExpanded({
  source,
  defaultValue,
  type = 'text',
  disabled = false,
  name,
  ...props
}: {
  source: string
  defaultValue: string | number
  label?: string
  type?: HTMLInputElement['type']
  disabled?: boolean
  name?: string
}) {
  const { id, field } = useInput({ source })

  React.useEffect(() => {
    if (
      !field.value ||
      typeof field.value !== (type === 'number' ? 'number' : 'string')
    ) {
      field.onChange(defaultValue)
    }
  }, [type, name, defaultValue])

  return (
    <TextField
      id={id}
      {...props}
      {...field}
      disabled={disabled}
      onChange={({ target }) => {
        field.onChange({
          target: {
            value: type === 'number' ? +target.value || 0 : target.value,
          },
        })
      }}
      type={type}
      value={(field.value ?? defaultValue) || (type === 'number' ? 0 : '')}
    />
  )
}

export function ColorInputExpanded({
  source,
  defaultValue,
  name,
  ...props
}: {
  source: string
  defaultValue: string
  name?: string
  label?: string
}) {
  const { field } = useInput({ source, defaultValue })

  React.useEffect(() => {
    if (
      !field.value ||
      typeof field.value !== 'string' ||
      !field.value.startsWith('#') ||
      field.value.startsWith('rgb')
    ) {
      field.onChange(defaultValue)
    }
  }, [name, defaultValue])

  return <ColorInput {...props} source={source} />
}
