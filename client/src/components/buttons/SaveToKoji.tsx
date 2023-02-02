import * as React from 'react'
import Button, { ButtonProps } from '@mui/material/Button'

import type { FeatureCollection } from '@assets/types'
import { refreshKojiCache, save } from '@services/fetches'

interface Props extends ButtonProps {
  fc: FeatureCollection
}

export default function SaveToKoji({ fc, ...rest }: Props) {
  const routes = {
    type: 'FeatureCollection',
    features: fc.features.filter((feat) => feat.geometry.type === 'MultiPoint'),
  }
  const fences = {
    type: 'FeatureCollection',
    features: fc.features.filter((feat) =>
      feat.geometry.type.includes('Polygon'),
    ),
  }
  return (
    <Button
      onClick={async () =>
        save('/api/v1/geofence/save-koji', JSON.stringify(fences))
          .then(() => save('/api/v1/route/save-koji', JSON.stringify(routes)))
          .then(() => refreshKojiCache())
      }
      {...rest}
    >
      Save to Kōji
    </Button>
  )
}
