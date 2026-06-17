import Button, { ButtonProps } from '@mui/material/Button'
import { getGolbatCache, save } from '@services/fetches'
import * as React from 'react'

interface Props extends ButtonProps {
  fc: string
}

export default function SaveToGolbat({ fc, ...rest }: Props) {
  const [loading, setLoading] = React.useState(false)
  return (
    <Button
      disabled={loading}
      onClick={async () => {
        setLoading(true)
        // TODO(v2-verify): the v1 `/geofence/save-golbat` (write drawn fences
        // straight into the golbat DB) has no direct v2 equivalent — v2 separates
        // "save to Kōji" (POST /api/v2/geofences) from "publish to golbat" (POST
        // /api/v2/geofences/{id}/publish, which needs an existing linked fence).
        // Best-effort: save the drawn features as Kōji geofences; publishing to
        // the golbat must then happen via the admin publish action. Flagged.
        await save('geofences', fc)
        await getGolbatCache()
        return setLoading(false)
      }}
      {...rest}
    >
      Save to Golbat
    </Button>
  )
}
