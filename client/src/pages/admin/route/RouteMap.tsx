import * as React from 'react'
import Map from '@components/Map'
import GeoJsonWrapper from '@components/GeojsonWrapper'
import { FeatureCollection, KojiRoute } from '@assets/types'
import type { MultiPoint } from 'geojson'
import { safeParse } from '@services/utils'

export default function RouteMap({ formData }: { formData: KojiRoute }) {
  const parsed: MultiPoint | null =
    typeof formData.geometry === 'string'
      ? (() => {
          const safe = safeParse<MultiPoint>(formData.geometry)
          if (!safe.error) return safe.value
          return null
        })()
      : formData.geometry

  const fc: FeatureCollection = {
    type: 'FeatureCollection',
    features: (parsed?.coordinates || []).map((c, i) => {
      return {
        id: `${i}`,
        type: 'Feature',
        geometry: {
          type: 'Point',
          coordinates: c,
        },
        properties: {
          next: parsed?.coordinates[
            i === (parsed?.coordinates?.length || 0) - 1 ? 0 : i + 1
          ],
        },
      }
    }),
  }
  return (
    <Map
      key={JSON.stringify(fc)}
      zoomControl
      style={{ width: '100%', height: '50vh' }}
    >
      <GeoJsonWrapper data={fc} mode={formData.mode} />
    </Map>
  )
}
