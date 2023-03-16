import * as React from 'react'
import type { Geometry } from 'geojson'

import type { AdminGeofence } from '@assets/types'
import Map from '@components/Map'
import GeoJsonWrapper from '@components/GeojsonWrapper'
import { safeParse } from '@services/utils'

export function GeofenceMap({ formData }: { formData: AdminGeofence }) {
  const parsed: Geometry | null =
    typeof formData.geometry === 'string'
      ? (() => {
          const safe = safeParse<Geometry>(formData.geometry)
          if (!safe.error) return safe.value
          return null
        })()
      : formData.geometry
  if (!parsed) return null
  return (
    <Map zoomControl style={{ width: '100%', height: '50vh' }}>
      <GeoJsonWrapper
        data={{
          type: 'FeatureCollection',
          features: [
            {
              id: '',
              type: 'Feature',
              properties: Object.fromEntries(
                (formData.properties || []).map((p) => [p.name, p.value]),
              ),
              geometry: parsed,
            },
          ],
        }}
      />
    </Map>
  )
}
