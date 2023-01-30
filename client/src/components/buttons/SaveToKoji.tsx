import Button, { ButtonProps } from '@mui/material/Button'
import { refreshKojiCache, save } from '@services/fetches'
import * as React from 'react'

interface Props extends ButtonProps {
  fc: string
}

export default function SaveToKoji({ fc, ...rest }: Props) {
  return (
    <Button
      onClick={() =>
        save('/api/v1/geofence/save-koji', fc).then(() => refreshKojiCache())
      }
      {...rest}
    >
      Save to Koji
    </Button>
  )
}
