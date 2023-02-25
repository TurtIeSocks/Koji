import { useStatic } from '@hooks/useStatic'
import Button, { ButtonProps } from '@mui/material/Button'
import { getScannerCache, save } from '@services/fetches'
import * as React from 'react'

interface Props extends ButtonProps {
  fc: string
}

export default function SaveToScanner({ fc, ...rest }: Props) {
  const [loading, setLoading] = React.useState(false)
  return (
    <Button
      disabled={!useStatic.getState().dangerous || loading}
      onClick={() => {
        setLoading(true)
        return save('/api/v1/geofence/save-scanner', fc)
          .then(() => getScannerCache())
          .then(() => setLoading(false))
      }}
      {...rest}
    >
      Save to Scanner
    </Button>
  )
}
