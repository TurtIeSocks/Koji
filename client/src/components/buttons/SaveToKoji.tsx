import Button, { ButtonProps } from '@mui/material/Button'
import { save } from '@services/fetches'
import * as React from 'react'

interface Props extends ButtonProps {
  fc: string
}

export default function SaveToKoji({ fc, ...rest }: Props) {
  return (
    <Button
      onClick={() =>
        // eslint-disable-next-line no-console
        save('/api/v1/geofence/save-koji', fc).then((res) => console.log(res))
      }
      {...rest}
    >
      Save to Koji
    </Button>
  )
}
