/* eslint-disable no-console */
import { useStatic } from '@hooks/useStatic'
import Button, { ButtonProps } from '@mui/material/Button'
import { save } from '@services/fetches'
import * as React from 'react'

interface Props extends ButtonProps {
  fc: string
}

export default function SaveToScanner({ fc, ...rest }: Props) {
  return (
    <Button
      disabled={!useStatic.getState().dangerous}
      onClick={() =>
        save('/api/v1/geofence/save-scanner', fc).then((res) =>
          console.log(res),
        )
      }
      {...rest}
    >
      Save to Scanner
    </Button>
  )
}
