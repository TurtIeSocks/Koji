import * as React from 'react'
import { GeoJSON, GeoJSONProps, useMap } from 'react-leaflet'
import distance from '@turf/distance'
import * as L from 'leaflet'

import type { FeatureCollection } from '@assets/types'
import { getColor, mpToPoints } from '@services/utils'

export default function GeoJsonWrapper({
  data,
  mode,
  ...rest
}: {
  data: FeatureCollection
  mode?: string
} & GeoJSONProps) {
  const map = useMap()
  const featureCollection: FeatureCollection = {
    ...data,
    features: data.features.flatMap((feat) =>
      feat.geometry.type === 'MultiPoint' ? mpToPoints(feat.geometry) : feat,
    ),
  }
  return (
    <GeoJSON
      data={featureCollection}
      onEachFeature={(feat, layer) => {
        layer.bindPopup(`
          <div>
            <ul>
              ${Object.entries(feat.properties || {})
                .map(([k, v]) => `<li>${k.replace('__', '')}: ${v}</li>`)
                .join('')}
            </ul>
          </div>`)
      }}
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
                circle_quest: 80,
                circle_raid: 1100,
                circle_smart_raid: 1100,
              }[mode] || 70
            : feat.properties?.radius || 70,
        })
      }}
      {...rest}
    />
  )
}
