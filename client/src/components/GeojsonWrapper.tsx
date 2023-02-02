import * as React from 'react'
import { GeoJSON, GeoJSONProps, useMap } from 'react-leaflet'
import distance from '@turf/distance'
import * as L from 'leaflet'

import type { FeatureCollection } from '@assets/types'
import { getColor, mpToPoints } from '@services/utils'

export default function GeoJsonWrapper({
  data: fc,
  mode,
  ...rest
}: {
  data: FeatureCollection
  mode?: string
} & GeoJSONProps) {
  const map = useMap()
  const featureCollection: FeatureCollection = {
    ...fc,
    features: fc.features.flatMap((feat) =>
      feat.geometry.type === 'MultiPoint' ? mpToPoints(feat.geometry) : feat,
    ),
  }
  return (
    <GeoJSON
      data={featureCollection}
      pointToLayer={(feat, latlng) => {
        if (feat.properties?.next) {
          L.polyline(
            [
              [latlng.lat, latlng.lng],
              [feat.properties.next[1], feat.properties.next[0]],
            ],
            {
              color: getColor(
                distance(feat, feat.properties.next, {
                  units: 'meters',
                }),
              ),
            },
          ).addTo(map)
        }
        return L.circle(latlng, {
          radius: mode
            ? {
                ManualQuest: 80,
                CircleRaid: 1100,
                CircleSmartRaid: 1100,
              }[mode] || 70
            : feat.properties?.radius || 70,
        })
      }}
      {...rest}
    />
  )
}