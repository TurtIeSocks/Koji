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
      disabled={useStatic.getState().scannerType !== 'unown' || loading}
      onClick={async () => {
        setLoading(true)
        await save('/api/v1/geofence/save-scanner', fc)
        await getScannerCache()
        return setLoading(false)
      }}
      {...rest}
    >
      Save to Scanner
    </Button>
  )
}
