import * as React from 'react'
import Button, { ButtonProps } from '@mui/material/Button'

import type { FeatureCollection } from '@assets/types'
import { getKojiCache, save } from '@services/fetches'

interface Props extends ButtonProps {
  fc: FeatureCollection
}

export default function SaveToKoji({ fc, ...rest }: Props) {
  const [loading, setLoading] = React.useState(false)
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
      disabled={loading}
      onClick={() => {
        setLoading(true)
        return save('/api/v1/geofence/save-koji', JSON.stringify(fences))
          .then(() => getKojiCache('geofence'))
          .then((newFences) =>
            save(
              '/api/v1/route/save-koji',
              JSON.stringify(
                routes.features.map((feat) => ({
                  ...feat,
                  properties: {
                    ...feat.properties,
                    __geofence_id: Object.values(newFences || {}).find(
                      (x) =>
                        x.name === feat.properties.__geofence_id?.toString(),
                    )?.id,
                  },
                })),
              ),
            ),
          )
          .then(() => getKojiCache('route').then(() => setLoading(false)))
      }}
      {...rest}
    >
      Save to Kōji
    </Button>
  )
}
